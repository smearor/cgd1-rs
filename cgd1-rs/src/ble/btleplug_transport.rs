use std::collections::HashMap;
use std::pin::Pin;
use std::sync::atomic::AtomicBool;
use std::sync::atomic::Ordering;

use async_trait::async_trait;
use btleplug::api::Central;
use btleplug::api::Manager as BtleplugManager;
use btleplug::api::Peripheral as BtleplugPeripheralApi;
use btleplug::api::ScanFilter;
use btleplug::api::ValueNotification;
use btleplug::api::WriteType;
use btleplug::platform::Adapter;
use btleplug::platform::Manager;
use futures::Stream;
use futures::stream::StreamExt;
use tokio::sync::Mutex;
use tracing::debug;
use uuid::Uuid;

use crate::ble::advertisement::AdvertisementData;
use crate::ble::characteristic::CharacteristicUuid;
use crate::ble::transport::BleTransport;
use crate::ble::transport_state::TransportState;
use crate::error::ClockError;
use crate::error::Result;
use crate::error::TransportError;
use crate::types::MacAddress;

type NotificationStream = Pin<Box<dyn Stream<Item = ValueNotification> + Send>>;
type EventStream = Pin<Box<dyn Stream<Item = btleplug::api::CentralEvent> + Send>>;

/// btleplug implementation of [`BleTransport`].
///
/// Wraps a `btleplug::platform::Adapter` and manages a single active BLE
/// connection. Scanning, notification, and connection state are tracked
/// in separate mutexes to avoid holding locks across await points.
pub struct BtleplugTransport {
    adapter: Adapter,
    state: Mutex<TransportState>,
    event_stream: Mutex<Option<EventStream>>,
    notification_stream: Mutex<Option<NotificationStream>>,
    connected: AtomicBool,
}

impl BtleplugTransport {
    /// Create a new transport by selecting the first available Bluetooth adapter.
    pub async fn new() -> Result<Self> {
        let manager = Manager::new().await.map_err(ClockError::from)?;
        let adapters = manager.adapters().await.map_err(ClockError::from)?;
        let adapter = adapters.into_iter().next().ok_or(TransportError::NoAdapter)?;
        Ok(Self {
            adapter,
            state: Mutex::new(TransportState::new()),
            event_stream: Mutex::new(None),
            notification_stream: Mutex::new(None),
            connected: AtomicBool::new(false),
        })
    }
}

#[async_trait]
impl BleTransport for BtleplugTransport {
    async fn start_scan(&self, filter_uuid: Uuid) -> Result<()> {
        {
            let mut state = self.state.lock().await;
            state.scan_filter_uuid = Some(filter_uuid);
        }

        // Ensure the event stream exists before scanning starts.
        {
            let mut stream_guard = self.event_stream.lock().await;
            if stream_guard.is_none() {
                let events = self.adapter.events().await.map_err(ClockError::from)?;
                *stream_guard = Some(Box::pin(events));
            }
        }

        // Start scanning with an empty filter — we filter manually in
        // `next_advertisement` because `ScanFilter` matches advertised
        // service UUIDs, not service-data UUIDs.
        self.adapter.start_scan(ScanFilter::default()).await.map_err(ClockError::from)?;
        Ok(())
    }

    async fn stop_scan(&self) -> Result<()> {
        self.adapter.stop_scan().await.map_err(ClockError::from)?;
        Ok(())
    }

    async fn next_advertisement(&self) -> Option<AdvertisementData> {
        let filter_uuid = {
            let state = self.state.lock().await;
            state.scan_filter_uuid
        };
        let filter_uuid = filter_uuid?;

        loop {
            let event = {
                let mut stream_guard = self.event_stream.lock().await;
                let stream = stream_guard.as_mut()?;
                stream.next().await
            };

            match event {
                Some(btleplug::api::CentralEvent::ServiceDataAdvertisement { service_data, .. }) => {
                    if let Some(payload) = service_data.get(&filter_uuid)
                        && let Ok(data) = AdvertisementData::parse(payload)
                    {
                        return Some(data);
                    }
                }
                _ => continue,
            }
        }
    }

    async fn connect(&self, address: &MacAddress) -> Result<()> {
        if self.connected.load(Ordering::SeqCst) {
            return Err(ClockError::AlreadyConnected);
        }

        let peripherals = self.adapter.peripherals().await.map_err(ClockError::from)?;

        let normalized_target = address.normalized();
        let target = peripherals
            .into_iter()
            .find(|p| {
                let addr = p.address().to_string().replace([':', '-'], "").to_lowercase();
                addr == normalized_target
            })
            .ok_or(TransportError::DeviceNotFound { address: *address })?;

        target.connect().await.map_err(ClockError::from)?;
        target.discover_services().await.map_err(ClockError::from)?;

        let mut characteristics = HashMap::new();
        for char in target.characteristics() {
            characteristics.insert(char.uuid, char);
        }

        let notifications = target.notifications().await.map_err(ClockError::from)?;

        {
            let mut state = self.state.lock().await;
            state.peripheral = Some(target);
            state.characteristics = characteristics;
        }
        {
            let mut stream_guard = self.notification_stream.lock().await;
            *stream_guard = Some(Box::pin(notifications));
        }

        self.connected.store(true, Ordering::SeqCst);
        Ok(())
    }

    async fn disconnect(&self) -> Result<()> {
        let peripheral = {
            let mut state = self.state.lock().await;
            state.peripheral.take()
        };
        {
            let mut stream_guard = self.notification_stream.lock().await;
            stream_guard.take();
        }
        if let Some(peripheral) = peripheral {
            peripheral.disconnect().await.map_err(ClockError::from)?;
        }
        {
            let mut state = self.state.lock().await;
            state.characteristics.clear();
        }
        self.connected.store(false, Ordering::SeqCst);
        Ok(())
    }

    async fn write(&self, characteristic: CharacteristicUuid, data: &[u8]) -> Result<()> {
        let (peripheral, char) = {
            let state = self.state.lock().await;
            let peripheral = state.peripheral.as_ref().ok_or(ClockError::NotConnected)?.clone();
            let char = state
                .characteristics
                .get(&characteristic.uuid())
                .ok_or(TransportError::CharacteristicNotFound { characteristic })?
                .clone();
            (peripheral, char)
        };
        peripheral.write(&char, data, WriteType::WithResponse).await.map_err(ClockError::from)?;
        Ok(())
    }

    async fn subscribe(&self, characteristic: CharacteristicUuid) -> Result<()> {
        let (peripheral, char) = {
            let state = self.state.lock().await;
            let peripheral = state.peripheral.as_ref().ok_or(ClockError::NotConnected)?.clone();
            let char = state
                .characteristics
                .get(&characteristic.uuid())
                .ok_or(TransportError::CharacteristicNotFound { characteristic })?
                .clone();
            (peripheral, char)
        };
        peripheral.subscribe(&char).await.map_err(ClockError::from)?;
        Ok(())
    }

    async fn next_notification(&self) -> Option<(Uuid, Vec<u8>)> {
        let mut stream_guard = self.notification_stream.lock().await;
        let stream = stream_guard.as_mut()?;
        let notification = stream.next().await?;
        Some((notification.uuid, notification.value))
    }

    async fn read(&self, characteristic: CharacteristicUuid) -> Result<Vec<u8>> {
        let (peripheral, char) = {
            let state = self.state.lock().await;
            let peripheral = state.peripheral.as_ref().ok_or(ClockError::NotConnected)?.clone();
            let char = state
                .characteristics
                .get(&characteristic.uuid())
                .ok_or(TransportError::CharacteristicNotFound { characteristic })?
                .clone();
            (peripheral, char)
        };
        let data = peripheral.read(&char).await.map_err(ClockError::from)?;
        Ok(data)
    }

    async fn request_mtu(&self, mtu: u16) -> Result<u16> {
        // btleplug does not expose a direct MTU exchange API.
        // On Linux (BlueZ), the MTU is automatically negotiated during
        // connection. We return the requested MTU as a best-effort
        // indication.
        debug!("request_mtu({mtu}) - btleplug does not expose MTU exchange, returning requested value");
        Ok(mtu)
    }

    fn is_connected(&self) -> bool {
        self.connected.load(Ordering::SeqCst)
    }
}
