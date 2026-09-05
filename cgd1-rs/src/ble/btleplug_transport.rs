use std::collections::HashMap;
use std::pin::Pin;
use std::sync::Arc;
use std::time::Duration;

use async_trait::async_trait;
use btleplug::api::Central;
use btleplug::api::CentralEvent;
use btleplug::api::Characteristic;
use btleplug::api::Manager as BtleplugManager;
use btleplug::api::Peripheral as BtleplugPeripheralApi;
use btleplug::api::ScanFilter;
use btleplug::api::ValueNotification;
use btleplug::api::WriteType;
use btleplug::platform::Adapter;
use btleplug::platform::Manager;
use btleplug::platform::Peripheral;
use futures::Stream;
use futures::stream::StreamExt;
use tokio::sync::Mutex;
use tracing::debug;
use tracing::info;
use tracing::trace;
use tracing::warn;
use uuid::Uuid;

use crate::ble::advertisement::AdvertisementData;
use crate::ble::characteristic::CharacteristicUuid;
use crate::ble::notification::BleNotification;
use crate::ble::transport::BleTransport;
use crate::ble::transport_state::ScanState;
use crate::error::ClockError;
use crate::error::Result;
use crate::error::TransportError;
use crate::types::MacAddress;

type NotificationStream = Pin<Box<dyn Stream<Item = ValueNotification> + Send>>;
type SharedNotificationStream = Arc<Mutex<NotificationStream>>;
type EventStream = Pin<Box<dyn Stream<Item = CentralEvent> + Send>>;

/// btleplug implementation of [`BleTransport`].
///
/// Wraps a `btleplug::platform::Adapter` and manages multiple simultaneous
/// BLE connections, keyed by MAC address. Each device has its own
/// peripheral, characteristics, and notification stream. Scanning is
/// global (shared across all devices).
pub struct BtleplugTransport {
    adapter: Adapter,
    scan_state: Mutex<ScanState>,
    event_stream: Mutex<Option<EventStream>>,
    connections: Mutex<HashMap<MacAddress, DeviceEntry>>,
    notification_streams: Mutex<HashMap<MacAddress, SharedNotificationStream>>,
}

/// Per-device connection entry holding the peripheral and characteristics.
struct DeviceEntry {
    peripheral: Peripheral,
    characteristics: HashMap<Uuid, Characteristic>,
}

impl BtleplugTransport {
    /// Create a new transport by selecting the first available Bluetooth adapter.
    pub async fn new() -> Result<Self> {
        let manager = Manager::new().await.map_err(ClockError::from)?;
        let adapters = manager.adapters().await.map_err(ClockError::from)?;
        let adapter = adapters.into_iter().next().ok_or(TransportError::NoAdapter)?;
        Ok(Self {
            adapter,
            scan_state: Mutex::new(ScanState::new()),
            event_stream: Mutex::new(None),
            connections: Mutex::new(HashMap::new()),
            notification_streams: Mutex::new(HashMap::new()),
        })
    }

    /// Look up the peripheral and characteristic for a connected device.
    async fn lookup_peripheral_and_char(&self, address: &MacAddress, characteristic: CharacteristicUuid) -> Result<(Peripheral, Characteristic)> {
        let connections = self.connections.lock().await;
        let entry = connections.get(address).ok_or(ClockError::NotConnected)?;
        let char = entry
            .characteristics
            .get(&characteristic.uuid())
            .ok_or(TransportError::CharacteristicNotFound { characteristic })?
            .clone();
        Ok((entry.peripheral.clone(), char))
    }

    /// Read and log standard GATT characteristics for device identification.
    ///
    /// Call this after authentication is complete to avoid polluting the
    /// notification stream with stale read responses before the notification
    /// task is ready.
    #[allow(dead_code)]
    async fn log_standard_gatt_characteristics(&self, address: &MacAddress) {
        if let Ok((peripheral, char)) = self.lookup_peripheral_and_char(address, CharacteristicUuid::DeviceName).await {
            if let Ok(data) = peripheral.read(&char).await {
                let name = String::from_utf8_lossy(&data);
                debug!(%address, device_name = %name, "GATT Device Name");
            }
        }
        if let Ok((peripheral, char)) = self.lookup_peripheral_and_char(address, CharacteristicUuid::Appearance).await {
            if let Ok(data) = peripheral.read(&char).await
                && data.len() >= 2
            {
                let appearance = u16::from_le_bytes([data[0], data[1]]);
                debug!(%address, appearance = appearance, "GATT Appearance");
            }
        }
        if let Ok((peripheral, char)) = self
            .lookup_peripheral_and_char(address, CharacteristicUuid::PeripheralPreferredConnectionParameters)
            .await
        {
            if let Ok(data) = peripheral.read(&char).await
                && data.len() >= 8
            {
                let min_interval = u16::from_le_bytes([data[0], data[1]]);
                let max_interval = u16::from_le_bytes([data[2], data[3]]);
                let slave_latency = u16::from_le_bytes([data[4], data[5]]);
                let supervision_timeout = u16::from_le_bytes([data[6], data[7]]);
                debug!(
                    %address,
                    min_interval_ms = min_interval as f32 * 1.25,
                    max_interval_ms = max_interval as f32 * 1.25,
                    slave_latency,
                    supervision_timeout_ms = supervision_timeout * 10,
                    "GATT Peripheral Preferred Connection Parameters"
                );
            }
        }
        if let Ok((peripheral, char)) = self.lookup_peripheral_and_char(address, CharacteristicUuid::PnpId).await {
            if let Ok(data) = peripheral.read(&char).await
                && data.len() >= 7
            {
                let vendor_id_source = data[0];
                let vendor_id = u16::from_le_bytes([data[1], data[2]]);
                let product_id = u16::from_le_bytes([data[3], data[4]]);
                let product_version = u16::from_le_bytes([data[5], data[6]]);
                debug!(
                    %address,
                    vendor_id_source,
                    vendor_id,
                    product_id,
                    product_version,
                    "GATT PnP ID"
                );
            }
        }
        if let Ok((peripheral, char)) = self.lookup_peripheral_and_char(address, CharacteristicUuid::FirmwareVersion).await {
            if let Ok(data) = peripheral.read(&char).await {
                let version = String::from_utf8_lossy(&data);
                debug!(%address, firmware_version = %version, "GATT Firmware Version");
            }
        }
    }

    /// Read and log all characteristics that are not in the known
    /// [`CharacteristicUuid`] enum. This helps discover undocumented
    /// characteristics during real-device connects.
    #[allow(dead_code)]
    async fn log_unknown_characteristics(&self, address: &MacAddress) {
        let unknown_uuids: Vec<Uuid> = {
            let connections = self.connections.lock().await;
            let Some(entry) = connections.get(address) else { return };
            entry
                .characteristics
                .keys()
                .copied()
                .filter(|uuid| CharacteristicUuid::try_from(*uuid).is_err())
                .collect()
        };

        for uuid in unknown_uuids {
            let (peripheral, char) = match self.lookup_peripheral_by_uuid(address, uuid).await {
                Ok(v) => v,
                Err(_) => continue,
            };
            match peripheral.read(&char).await {
                Ok(data) => {
                    debug!(
                        %address,
                        uuid = %uuid,
                        len = data.len(),
                        data = %format_hex(&data),
                        "unknown characteristic read"
                    );
                }
                Err(e) => {
                    debug!(
                        %address,
                        uuid = %uuid,
                        error = ?e,
                        "unknown characteristic read failed (write-only or notify-only)"
                    );
                }
            }
        }
    }

    /// Look up the peripheral and a raw characteristic by UUID.
    #[allow(dead_code)]
    async fn lookup_peripheral_by_uuid(&self, address: &MacAddress, uuid: Uuid) -> Result<(Peripheral, Characteristic)> {
        let connections = self.connections.lock().await;
        let entry = connections.get(address).ok_or(ClockError::NotConnected)?;
        let char = entry.characteristics.get(&uuid).cloned().ok_or(TransportError::CharacteristicNotFound {
            characteristic: CharacteristicUuid::AuthWrite,
        })?;
        Ok((entry.peripheral.clone(), char))
    }
}

#[async_trait]
impl BleTransport for BtleplugTransport {
    async fn start_scan(&self, filter_uuid: Uuid) -> Result<()> {
        debug!(%filter_uuid, "starting BLE scan");
        {
            let mut scan_state = self.scan_state.lock().await;
            scan_state.scan_filter_uuid = Some(filter_uuid);
        }

        // Ensure the event stream exists before scanning starts.
        {
            let mut stream_guard = self.event_stream.lock().await;
            if stream_guard.is_none() {
                let events = self.adapter.events().await.map_err(ClockError::from)?;
                *stream_guard = Some(Box::pin(events));
            }
        }

        // Start scanning with an empty filter - we filter manually in
        // `next_advertisement` because `ScanFilter` matches advertised
        // service UUIDs, not service-data UUIDs.
        self.adapter.start_scan(ScanFilter::default()).await.map_err(ClockError::from)?;
        debug!("BLE scan started");
        Ok(())
    }

    async fn stop_scan(&self) -> Result<()> {
        debug!("stopping BLE scan");
        self.adapter.stop_scan().await.map_err(ClockError::from)?;
        debug!("BLE scan stopped");
        Ok(())
    }

    async fn next_advertisement(&self) -> Option<AdvertisementData> {
        let filter_uuid = {
            let scan_state = self.scan_state.lock().await;
            scan_state.scan_filter_uuid
        };
        let filter_uuid = filter_uuid?;

        loop {
            let event = {
                let mut stream_guard = self.event_stream.lock().await;
                let stream = stream_guard.as_mut()?;
                stream.next().await
            };

            match event {
                Some(CentralEvent::ServiceDataAdvertisement { service_data, .. }) => {
                    if let Some(payload) = service_data.get(&filter_uuid)
                        && let Ok(data) = AdvertisementData::parse(payload)
                    {
                        debug!(
                            mac = %data.mac,
                            temp = data.temperature.value(),
                            humidity = data.humidity.value(),
                            battery = data.battery.value(),
                            payload_len = payload.len(),
                            "advertisement matched filter"
                        );
                        return Some(data);
                    }
                }
                Some(ref ev) => {
                    trace!(event = ?ev, "ignoring non-matching advertisement event");
                }
                _ => continue,
            }
        }
    }

    async fn connect(&self, address: &MacAddress) -> Result<()> {
        debug!(%address, "connect attempt");

        // Check if already connected.
        {
            let connections = self.connections.lock().await;
            if connections.contains_key(address) {
                warn!(%address, "connect called but already connected");
                return Err(ClockError::AlreadyConnected);
            }
        }

        let normalized_target = address.normalized();

        // Retry loop: the peripheral may not be in BlueZ's cache yet,
        // especially on startup when a parallel scan is still running.
        // Wait and retry a few times before giving up.
        let target = {
            let mut attempts = 0u32;
            const MAX_ATTEMPTS: u32 = 5;
            const RETRY_DELAY: Duration = Duration::from_secs(2);

            loop {
                let peripherals = self.adapter.peripherals().await.map_err(ClockError::from)?;
                debug!(peripheral_count = peripherals.len(), %address, attempt = attempts + 1, "discovered peripherals");

                if let Some(p) = peripherals.into_iter().find(|p| {
                    let addr = p.address().to_string().replace([':', '-'], "").to_lowercase();
                    addr == normalized_target
                }) {
                    break p;
                }

                attempts += 1;
                if attempts >= MAX_ATTEMPTS {
                    return Err(TransportError::DeviceNotFound { address: *address }.into());
                }
                debug!(%address, attempt = attempts, max = MAX_ATTEMPTS, "peripheral not in cache, retrying after delay");
                tokio::time::sleep(RETRY_DELAY).await;
            }
        };

        debug!(%address, "peripheral found, connecting");
        target.connect().await.map_err(ClockError::from)?;
        debug!(%address, "BLE connected, discovering services");
        target.discover_services().await.map_err(ClockError::from)?;

        let mut characteristics = HashMap::new();
        for char in target.characteristics() {
            debug!(%address, uuid = %char.uuid, "discovered characteristic");
            characteristics.insert(char.uuid, char);
        }
        debug!(char_count = characteristics.len(), %address, "services discovered");

        let notifications = target.notifications().await.map_err(ClockError::from)?;

        {
            let mut connections = self.connections.lock().await;
            connections.insert(
                *address,
                DeviceEntry {
                    peripheral: target,
                    characteristics,
                },
            );
        }
        {
            let mut streams = self.notification_streams.lock().await;
            streams.insert(*address, Arc::new(Mutex::new(Box::pin(notifications))));
        }

        info!(%address, "BLE connection established");
        Ok(())
    }

    async fn disconnect(&self, address: &MacAddress) -> Result<()> {
        debug!(%address, "disconnect attempt");
        let entry = {
            let mut connections = self.connections.lock().await;
            connections.remove(address)
        };
        {
            let mut streams = self.notification_streams.lock().await;
            streams.remove(address);
        }

        if let Some(entry) = entry {
            // Timeout the peripheral disconnect - BlueZ can hang indefinitely
            // on disconnect if the device is unresponsive, which blocks all
            // subsequent adapter operations (peripherals(), scan, etc).
            match tokio::time::timeout(Duration::from_secs(5), entry.peripheral.disconnect()).await {
                Ok(Ok(())) => info!(%address, "BLE disconnected"),
                Ok(Err(e)) => warn!(%address, "peripheral.disconnect() failed, state already cleared: {e:?}"),
                Err(_) => warn!(%address, "peripheral.disconnect() timed out after 5s, state already cleared"),
            }
        } else {
            debug!(%address, "disconnect: no active connection for this device");
        }
        Ok(())
    }

    async fn write(&self, address: &MacAddress, characteristic: CharacteristicUuid, data: &[u8]) -> Result<()> {
        debug!(
            %address,
            characteristic = %characteristic,
            len = data.len(),
            data = %format_hex(data),
            "write -> device"
        );
        let (peripheral, char) = self.lookup_peripheral_and_char(address, characteristic).await?;
        peripheral.write(&char, data, WriteType::WithResponse).await.map_err(ClockError::from)?;
        Ok(())
    }

    async fn subscribe(&self, address: &MacAddress, characteristic: CharacteristicUuid) -> Result<()> {
        debug!(%address, characteristic = %characteristic, "subscribing to characteristic");
        let (peripheral, char) = self.lookup_peripheral_and_char(address, characteristic).await?;
        peripheral.subscribe(&char).await.map_err(ClockError::from)?;
        debug!(%address, characteristic = %characteristic, "subscribed to characteristic");
        Ok(())
    }

    async fn next_notification(&self, address: &MacAddress) -> Option<BleNotification> {
        let stream_arc = {
            let streams = self.notification_streams.lock().await;
            streams.get(address).cloned()
        }?;
        let mut stream = stream_arc.lock().await;
        let notification = stream.next().await?;
        let characteristic = match CharacteristicUuid::try_from(notification.uuid) {
            Ok(c) => c,
            Err(uuid) => {
                warn!(%address, %uuid, len = notification.value.len(), data = %format_hex(&notification.value), "notification from unknown characteristic, skipping");
                return self.next_notification(address).await;
            }
        };
        debug!(
            %address,
            characteristic = %characteristic,
            len = notification.value.len(),
            data = %format_hex(&notification.value),
            "notification <- device"
        );
        Some(BleNotification::new(characteristic, notification.value))
    }

    async fn read(&self, address: &MacAddress, characteristic: CharacteristicUuid) -> Result<Vec<u8>> {
        debug!(%address, characteristic = %characteristic, "reading characteristic");
        let (peripheral, char) = self.lookup_peripheral_and_char(address, characteristic).await?;
        let data = peripheral.read(&char).await.map_err(ClockError::from)?;
        debug!(
            %address,
            characteristic = %characteristic,
            len = data.len(),
            data = %format_hex(&data),
            "read <- device"
        );
        Ok(data)
    }

    async fn request_mtu(&self, _address: &MacAddress, mtu: u16) -> Result<u16> {
        // btleplug does not expose a direct MTU exchange API.
        // On Linux (BlueZ), the MTU is automatically negotiated during
        // connection. We return the requested MTU as a best-effort
        // indication.
        debug!("request_mtu({mtu}) - btleplug does not expose MTU exchange, returning requested value");
        Ok(mtu)
    }

    fn is_connected(&self, address: &MacAddress) -> bool {
        if let Ok(connections) = self.connections.try_lock() {
            connections.contains_key(address)
        } else {
            false
        }
    }
}

/// Format a byte slice as a hex string for debug logging.
pub fn format_hex(data: &[u8]) -> String {
    if data.len() <= 64 {
        data.iter().map(|b| format!("{b:02x}")).collect::<Vec<_>>().join(" ")
    } else {
        let head: String = data[..32].iter().map(|b| format!("{b:02x}")).collect::<Vec<_>>().join(" ");
        let tail: String = data[data.len() - 16..].iter().map(|b| format!("{b:02x}")).collect::<Vec<_>>().join(" ");
        format!("{head} ... ({len} bytes) ... {tail}", len = data.len())
    }
}
