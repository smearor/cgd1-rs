use std::collections::HashMap;
use std::sync::Arc;
use std::time::Instant;
use tokio::sync::Mutex;

use async_trait::async_trait;
use tokio::sync::mpsc;
use uuid::Uuid;

use super::device_state::ALARM_SLOT_COUNT;
use super::device_state::VirtualDeviceState;
use crate::AdvertisementData;
use crate::BatteryLevel;
use crate::CharacteristicUuid;
use crate::ClockTime;
use crate::Humidity;
use crate::MacAddress;
use crate::SensorNotification;
use crate::Temperature;
use crate::ble::transport::BleTransport;
use crate::command::AlarmEntry;
use crate::command::AlarmSlotIndex;
use crate::command::Brightness;
use crate::command::Command;
use crate::command::CommandId;
use crate::command::DeviceSettings;
use crate::command::Language;
use crate::command::RingtoneSignature;
use crate::command::ScreenLightDuration;
use crate::command::TemperatureUnit;
use crate::command::TimeFormat;
use crate::command::Timezone;
use crate::command::Volume;
use crate::error::Result;
use crate::error::TransportError;

/// Firmware version reported by the virtual device.
const VIRTUAL_FIRMWARE: &str = "1.0.0-virtual";

/// Alarms per response packet (3 entries x 5 bytes + 3 header bytes).
const ALARMS_PER_PACKET: usize = 3;

/// Default virtual device MAC addresses.
const DEFAULT_VIRTUAL_MACS: [&str; 5] = [
    "AA:BB:CC:DD:E0:01",
    "AA:BB:CC:DD:E0:02",
    "AA:BB:CC:DD:E0:03",
    "AA:BB:CC:DD:E0:04",
    "AA:BB:CC:DD:E0:05",
];

/// A virtual CGD1 alarm clock for testing CLI and WS without real hardware.
///
/// Implements the `BleTransport` trait with a full in-memory device simulation:
/// - Authentication handshake (accepts any 16-byte token).
/// - Alarm read/write/delete with 16 slots.
/// - Settings read/write with the same 18-byte encoding as the real device.
/// - Battery level via GATT read.
/// - Firmware version string.
/// - Time sync (accepted, no-op).
/// - Brightness preview (accepted, no-op).
/// - Ringtone preview (accepted, no-op).
/// - Audio upload (accepted, discards data).
/// - Sensor notifications (periodic, spawned on connect).
/// - Scan results (one pre-configured device).
///
/// Supports multiple simultaneous connections, each with its own
/// notification channel and sensor task.
pub struct VirtualClockTransport {
    /// Per-device connection state and notification channels.
    connections: Mutex<HashMap<MacAddress, VirtualConnection>>,
    /// All known virtual devices.
    devices: Arc<Mutex<HashMap<MacAddress, Arc<Mutex<VirtualDeviceState>>>>>,
    /// Advertisements for scanning.
    advertisements: Mutex<Vec<AdvertisementData>>,
    scan_index: Mutex<usize>,
}

/// Per-device connection state for the virtual transport.
struct VirtualConnection {
    /// Notification channel sender for this specific device.
    notifications_tx: mpsc::UnboundedSender<(Uuid, Vec<u8>)>,
    /// Notification channel receiver, wrapped in Arc<Mutex> so it can be
    /// cloned and awaited without holding the connections lock.
    notifications_rx: Arc<Mutex<mpsc::UnboundedReceiver<(Uuid, Vec<u8>)>>>,
    /// Sensor task handle for this connection.
    sensor_task_handle: Option<tokio::task::JoinHandle<()>>,
    /// Subscribed characteristics for this connection.
    subscribed: Vec<CharacteristicUuid>,
}

impl VirtualClockTransport {
    /// Create new virtual transport with 5 default devices.
    pub fn new() -> Self {
        let mut devices = HashMap::new();
        for mac_str in DEFAULT_VIRTUAL_MACS {
            if let Ok(mac) = MacAddress::parse(mac_str) {
                devices.insert(mac, Arc::new(Mutex::new(VirtualDeviceState::default())));
            }
        }
        Self {
            connections: Mutex::new(HashMap::new()),
            devices: Arc::new(Mutex::new(devices)),
            advertisements: Mutex::new(Vec::new()),
            scan_index: Mutex::new(0),
        }
    }

    /// Create a virtual transport with 5 devices that have differentiated
    /// sensor values, settings, and battery levels — so multi-device behavior
    /// is visually verifiable in the UI and testable.
    ///
    /// | Device | MAC Suffix | Temp | Humidity | Battery | Time Format | Timezone | Language |
    /// |--------|------------|------|----------|---------|-------------|----------|----------|
    /// | 1      | `:01`      | 27.7 | 50.3     | 100     | 24h         | +2       | English  |
    /// | 2      | `:02`      | 22.1 | 45.0     | 87      | 24h         | +2       | English  |
    /// | 3      | `:03`      | 18.5 | 60.0     | 65      | 24h         | +8       | Chinese  |
    /// | 4      | `:04`      | 15.0 | 30.0     | 20      | 12h         | +2       | English  |
    /// | 5      | `:05`      | 30.2 | 70.0     | 100     | 12h         | -5       | English  |
    pub fn new_differentiated() -> Self {
        let configs = [
            // (mac_suffix, temp, humidity, battery, time_format, timezone_hours, language)
            (1u8, 27.7, 50.3, 100, TimeFormat::TwentyFourHour, 2, Language::English),
            (2, 22.1, 45.0, 87, TimeFormat::TwentyFourHour, 2, Language::English),
            (3, 18.5, 60.0, 65, TimeFormat::TwentyFourHour, 8, Language::Chinese),
            (4, 15.0, 30.0, 20, TimeFormat::TwelveHour, 2, Language::English),
            (5, 30.2, 70.0, 100, TimeFormat::TwelveHour, -5, Language::English),
        ];

        let mut devices = HashMap::new();
        for (suffix, temp, humidity, battery, time_format, tz_hours, language) in configs {
            let mac_str = format!("AA:BB:CC:DD:E0:{:02X}", suffix);
            if let Ok(mac) = MacAddress::parse(&mac_str) {
                let state = VirtualDeviceState {
                    temperature: Temperature::new(temp),
                    humidity: Humidity::new(humidity),
                    battery: BatteryLevel::new(battery),
                    settings: DeviceSettings::new(
                        Volume::new(3).unwrap(),
                        time_format,
                        TemperatureUnit::Celsius,
                        language,
                        Timezone::from_hours(tz_hours).unwrap(),
                        ScreenLightDuration::new(10).unwrap(),
                        Brightness::new(80).unwrap(),
                        Brightness::new(30).unwrap(),
                        ClockTime::new(22, 0).unwrap(),
                        ClockTime::new(7, 0).unwrap(),
                        true,
                        false,
                        RingtoneSignature::Unused,
                    )
                    .unwrap(),
                    ..Default::default()
                };
                devices.insert(mac, Arc::new(Mutex::new(state)));
            }
        }
        Self {
            connections: Mutex::new(HashMap::new()),
            devices: Arc::new(Mutex::new(devices)),
            advertisements: Mutex::new(Vec::new()),
            scan_index: Mutex::new(0),
        }
    }

    /// Create a virtual transport pre-loaded with an advertisement for the
    /// given MAC address, so `scan` returns a result immediately.
    pub fn with_advertisement(mac: MacAddress) -> Self {
        let transport = Self::new();
        transport.add_advertisement(AdvertisementData {
            mac,
            temperature: Temperature::new(22.5),
            humidity: Humidity::new(55.0),
            battery: BatteryLevel::new(85),
        });
        transport
    }

    /// Add an advertisement that will be returned during scanning.
    pub fn add_advertisement(&self, adv: AdvertisementData) {
        if let Ok(mut ads) = self.advertisements.try_lock() {
            ads.push(adv);
        }
    }

    /// Get the state of the device identified by MAC address.
    ///
    /// Returns an error if the MAC is unknown.
    async fn device_state(&self, address: &MacAddress) -> Result<Arc<Mutex<VirtualDeviceState>>> {
        let devices = self.devices.lock().await;
        devices.get(address).cloned().ok_or(TransportError::UnknownDeviceMac { mac: *address }.into())
    }

    /// Get the state of the currently connected device (first connection).
    ///
    /// Convenience method for test helpers. Returns an error if no device
    /// is connected.
    #[cfg(test)]
    async fn connected_state(&self) -> Result<Arc<Mutex<VirtualDeviceState>>> {
        let connections = self.connections.lock().await;
        let mac = connections.keys().next().copied().ok_or(TransportError::NotConnected)?;
        drop(connections);
        self.device_state(&mac).await
    }

    /// Set the battery level of a specific device.
    pub async fn set_battery(&self, address: &MacAddress, level: u8) {
        if let Ok(state_arc) = self.device_state(address).await {
            let mut state = state_arc.lock().await;
            state.battery = BatteryLevel::new(level);
        }
    }

    /// Set the sensor values of a specific device.
    pub async fn set_sensor_values(&self, address: &MacAddress, temperature: f32, humidity: f32) {
        if let Ok(state_arc) = self.device_state(address).await {
            let mut state = state_arc.lock().await;
            state.temperature = Temperature::new(temperature);
            state.humidity = Humidity::new(humidity);
        }
    }

    /// Set an alarm in a specific device's state directly.
    pub async fn set_alarm(&self, address: &MacAddress, slot: AlarmSlotIndex, entry: AlarmEntry) {
        if let Ok(state_arc) = self.device_state(address).await {
            let mut state = state_arc.lock().await;
            state.alarms[slot.value() as usize] = Some(entry);
        }
    }

    /// Get the current device time as a Unix timestamp.
    ///
    /// Returns `synced_time + elapsed_seconds` since the last Time Sync.
    /// Returns `None` if no Time Sync has been performed.
    pub async fn device_time(&self, address: &MacAddress) -> Option<u32> {
        let state_arc = self.device_state(address).await.ok()?;
        let state = state_arc.lock().await;
        let synced_time = state.synced_time?;
        let synced_at = state.synced_at?;
        let elapsed = synced_at.elapsed().as_secs() as u32;
        Some(synced_time + elapsed)
    }

    /// Push a sensor notification manually (for testing event subscription).
    pub fn push_sensor_notification(&self, address: &MacAddress, temperature: f32, humidity: f32) {
        let sensor = SensorNotification::new(Temperature::new(temperature), Humidity::new(humidity));
        self.push_notification(address, CharacteristicUuid::SensorNotify.uuid(), sensor.encode());
    }

    /// Push a sensor notification with battery manually (for testing).
    pub fn push_sensor_notification_with_battery(&self, address: &MacAddress, temperature: f32, humidity: f32, battery: u8) {
        let sensor = SensorNotification::with_battery(Temperature::new(temperature), Humidity::new(humidity), BatteryLevel::new(battery));
        self.push_notification(address, CharacteristicUuid::SensorNotify.uuid(), sensor.encode());
    }

    /// Push a battery notification manually (for testing event subscription).
    pub fn push_battery_notification(&self, address: &MacAddress, level: u8) {
        self.push_notification(address, CharacteristicUuid::BatteryLevel.uuid(), vec![level]);
    }

    /// Push a raw notification onto the notification channel for a specific device.
    fn push_notification(&self, address: &MacAddress, uuid: Uuid, data: Vec<u8>) {
        if let Ok(connections) = self.connections.try_lock() {
            if let Some(conn) = connections.get(address) {
                let _ = conn.notifications_tx.send((uuid, data));
            }
        }
    }

    /// Get the notification receiver Arc for a device without holding the
    /// connections lock during `recv()`.
    async fn notification_rx(&self, address: &MacAddress) -> Option<Arc<Mutex<mpsc::UnboundedReceiver<(Uuid, Vec<u8>)>>>> {
        let connections = self.connections.lock().await;
        connections.get(address).map(|conn| conn.notifications_rx.clone())
    }

    /// Send an ACK frame on the given notify characteristic for a specific device.
    fn send_ack_on(&self, address: &MacAddress, notify_char: CharacteristicUuid, command_byte: u8, status: u8, payload: u8) {
        let ack = vec![0x04, 0xff, command_byte, status, payload];
        self.push_notification(address, notify_char.uuid(), ack);
    }

    /// Send a success ACK on the given notify characteristic for a specific device.
    fn send_success_ack_on(&self, address: &MacAddress, notify_char: CharacteristicUuid, command_byte: u8) {
        self.send_ack_on(address, notify_char, command_byte, 0x00, 0x00);
    }

    /// Send a data notification (non-ACK) on the Data Notify characteristic.
    fn send_data_notification(&self, address: &MacAddress, data: Vec<u8>) {
        self.push_notification(address, CharacteristicUuid::DataNotify.uuid(), data);
    }

    /// Send a data notification on the Auth Notify characteristic.
    fn send_auth_notification(&self, address: &MacAddress, data: Vec<u8>) {
        self.push_notification(address, CharacteristicUuid::AuthNotify.uuid(), data);
    }

    /// Handle a command frame written to Auth Write.
    async fn handle_auth_write(&self, address: &MacAddress, data: &[u8]) {
        if data.len() < 2 {
            return;
        }
        let command_id = CommandId::new(data[1]);
        let payload = &data[2..];
        let notify = CharacteristicUuid::AuthNotify;

        let command = match Command::from_id_for_characteristic(command_id, CharacteristicUuid::AuthWrite) {
            Some(cmd) => cmd,
            None => {
                self.send_ack_on(address, notify, command_id.value(), 0x01, 0x00);
                return;
            }
        };

        match command {
            Command::AuthInit => {
                // Auth Init — accept any 16-byte token.
                if let Ok(state_arc) = self.device_state(address).await {
                    let mut state = state_arc.lock().await;
                    if payload.len() >= 16 {
                        let mut token = [0u8; 16];
                        token.copy_from_slice(&payload[..16]);
                        state.token = Some(token);
                    }
                }
                self.send_success_ack_on(address, notify, command_id.value());
            }
            Command::AuthConfirm => {
                // Auth Confirm — accept and mark as authenticated.
                if let Ok(state_arc) = self.device_state(address).await {
                    let mut state = state_arc.lock().await;
                    state.authenticated = true;
                }
                self.send_success_ack_on(address, notify, command_id.value());
            }
            Command::TimeSync => {
                // Time Sync — store the timestamp and the instant it was set.
                if payload.len() >= 4 {
                    let timestamp = u32::from_le_bytes([payload[0], payload[1], payload[2], payload[3]]);
                    if let Ok(state_arc) = self.device_state(address).await {
                        let mut state = state_arc.lock().await;
                        state.synced_time = Some(timestamp);
                        state.synced_at = Some(Instant::now());
                    }
                    self.send_success_ack_on(address, notify, command_id.value());
                } else {
                    self.send_ack_on(address, notify, command_id.value(), 0x01, 0x00);
                }
            }
            Command::ReadFirmware => {
                // Read Firmware — respond with version string on Auth Notify.
                let version_bytes = VIRTUAL_FIRMWARE.as_bytes();
                let mut response = Vec::with_capacity(2 + version_bytes.len());
                response.push((1 + version_bytes.len()) as u8);
                response.push(command_id.value());
                response.extend_from_slice(version_bytes);
                self.send_auth_notification(address, response);
            }
            _ => {
                self.send_ack_on(address, notify, command_id.value(), 0x01, 0x00);
            }
        }
    }

    /// Handle a command frame written to Data Write.
    async fn handle_data_write(&self, address: &MacAddress, data: &[u8]) {
        if data.len() < 2 {
            return;
        }
        let command_id = CommandId::new(data[1]);
        let payload = &data[2..];
        let notify = CharacteristicUuid::DataNotify;

        let command = match Command::from_id_for_characteristic(command_id, CharacteristicUuid::DataWrite) {
            Some(cmd) => cmd,
            None => {
                self.send_ack_on(address, notify, command_id.value(), 0x01, 0x00);
                return;
            }
        };

        match command {
            Command::SetSettings => {
                // Set Settings — decode and store.
                if payload.len() >= 18 {
                    if let Ok(settings) = DeviceSettings::decode(payload) {
                        if let Ok(state_arc) = self.device_state(address).await {
                            let mut state = state_arc.lock().await;
                            state.settings = settings;
                        }
                        self.send_success_ack_on(address, notify, command_id.value());
                    } else {
                        self.send_ack_on(address, notify, command_id.value(), 0x01, 0x00);
                    }
                } else {
                    self.send_ack_on(address, notify, command_id.value(), 0x01, 0x00);
                }
            }
            Command::ReadSettings => {
                // Read Settings — respond with encoded settings on Data Notify.
                if let Ok(state_arc) = self.device_state(address).await {
                    let state = state_arc.lock().await;
                    let encoded = state.settings.encode();
                    drop(state);
                    let mut response = Vec::with_capacity(20);
                    response.push(0x13);
                    response.push(command_id.value());
                    response.extend_from_slice(&encoded);
                    self.send_data_notification(address, response);
                }
            }
            Command::SetBrightness => {
                // Set Brightness — accept, no-op.
                self.send_success_ack_on(address, notify, command_id.value());
            }
            Command::PreviewRingtone => {
                // Preview Ringtone — accept, no-op.
                self.send_success_ack_on(address, notify, command_id.value());
            }
            Command::SetAlarm => {
                // Set/Delete Alarm.
                if payload.len() >= 6 {
                    let slot_byte = payload[0];
                    let entry_bytes = &payload[1..6];

                    if entry_bytes.iter().all(|&b| b == 0xFF) {
                        if let Ok(slot) = AlarmSlotIndex::new(slot_byte) {
                            if let Ok(state_arc) = self.device_state(address).await {
                                let mut state = state_arc.lock().await;
                                state.alarms[slot.value() as usize] = None;
                            }
                            self.send_success_ack_on(address, notify, command_id.value());
                        } else {
                            self.send_ack_on(address, notify, command_id.value(), 0x01, 0x00);
                        }
                    } else if let Ok(Some(entry)) = AlarmEntry::decode(entry_bytes) {
                        if let Ok(slot) = AlarmSlotIndex::new(slot_byte) {
                            if let Ok(state_arc) = self.device_state(address).await {
                                let mut state = state_arc.lock().await;
                                state.alarms[slot.value() as usize] = Some(entry);
                            }
                            self.send_success_ack_on(address, notify, command_id.value());
                        } else {
                            self.send_ack_on(address, notify, command_id.value(), 0x01, 0x00);
                        }
                    } else {
                        self.send_ack_on(address, notify, command_id.value(), 0x01, 0x00);
                    }
                } else {
                    self.send_ack_on(address, notify, command_id.value(), 0x01, 0x00);
                }
            }
            Command::ReadAlarms => {
                // Read Alarms — send 6 packets with 3 entries each.
                if let Ok(state_arc) = self.device_state(address).await {
                    let state = state_arc.lock().await;
                    let alarms: Vec<Option<AlarmEntry>> = state.alarms.clone();
                    drop(state);

                    let packets = ALARM_SLOT_COUNT.div_ceil(ALARMS_PER_PACKET);
                    for packet_idx in 0..packets {
                        let base_index = (packet_idx * ALARMS_PER_PACKET) as u8;
                        let mut response = Vec::with_capacity(20);
                        response.push(0x11);
                        response.push(command_id.value());
                        response.push(base_index);

                        for i in 0..ALARMS_PER_PACKET {
                            let slot_idx = packet_idx * ALARMS_PER_PACKET + i;
                            if slot_idx < ALARM_SLOT_COUNT {
                                match &alarms[slot_idx] {
                                    Some(entry) => response.extend_from_slice(&entry.encode()),
                                    None => response.extend_from_slice(&[0xFF; 5]),
                                }
                            } else {
                                response.extend_from_slice(&[0xFF; 5]);
                            }
                        }
                        self.send_data_notification(address, response);
                    }
                }
            }
            Command::AudioInit => {
                // Audio Init — accept, start upload.
                if payload.len() >= 7 {
                    let total_size = u32::from_le_bytes([payload[0], payload[1], payload[2], 0]) as usize;
                    if let Ok(state_arc) = self.device_state(address).await {
                        let mut state = state_arc.lock().await;
                        state.audio_upload_active = true;
                        state.audio_upload_total = total_size;
                        state.audio_upload_received = 0;
                        state.audio_block_packets = 0;
                    }
                    self.send_ack_on(address, notify, command_id.value(), 0x00, 0x00);
                } else {
                    self.send_ack_on(address, notify, command_id.value(), 0x01, 0x00);
                }
            }
            Command::AudioData => {
                // Audio Data Packet — accept, track progress.
                if let Ok(state_arc) = self.device_state(address).await {
                    let mut state = state_arc.lock().await;
                    if state.audio_upload_active {
                        let payload_len = payload.len();
                        state.audio_upload_received += payload_len;
                        state.audio_block_packets += 1;

                        let block_done = state.audio_block_packets >= 4 || state.audio_upload_received >= state.audio_upload_total;
                        let upload_done = state.audio_upload_received >= state.audio_upload_total;

                        if upload_done {
                            state.audio_upload_active = false;
                        }
                        if block_done {
                            state.audio_block_packets = 0;
                        }
                        drop(state);

                        if block_done {
                            self.send_success_ack_on(address, notify, command_id.value());
                        }
                    } else {
                        drop(state);
                        self.send_ack_on(address, notify, command_id.value(), 0x01, 0x00);
                    }
                }
            }
            _ => {
                self.send_ack_on(address, notify, command_id.value(), 0x01, 0x00);
            }
        }
    }

    /// Start a background task that periodically sends sensor notifications
    /// with slight temperature/humidity drift and battery drain for a
    /// specific connected device.
    fn start_sensor_task(&self, address: MacAddress) {
        // Get the notification sender for this device.
        let tx = {
            let connections = self.connections.try_lock();
            match connections {
                Ok(conns) => match conns.get(&address) {
                    Some(conn) => conn.notifications_tx.clone(),
                    None => return,
                },
                Err(_) => return,
            }
        };
        let sensor_uuid = CharacteristicUuid::SensorNotify.uuid();
        let battery_uuid = CharacteristicUuid::BatteryLevel.uuid();
        let devices = self.devices.clone();

        let handle = tokio::spawn(async move {
            let mut interval = tokio::time::interval(std::time::Duration::from_secs(5));
            interval.tick().await; // skip first immediate tick
            let mut tick_count: u64 = 0;
            // 5 min = 300 s = 60 ticks at 5 s each.
            const BATTERY_DRAIN_INTERVAL: u64 = 60;
            const BATTERY_LOW_THRESHOLD: u8 = 20;
            const BATTERY_RESET_LEVEL: u8 = 80;
            loop {
                interval.tick().await;
                tick_count += 1;

                // Get the connected device's state.
                let devices_map = devices.lock().await;
                let state_arc = match devices_map.get(&address) {
                    Some(s) => s.clone(),
                    None => continue,
                };
                drop(devices_map);
                let mut state = state_arc.lock().await;

                // Simulate small sensor drift (±0.1°C / ±0.1% per tick).
                let temp_drift = ((tick_count as f32 * 0.37).sin() * 0.1) + ((tick_count as f32 * 0.13).cos() * 0.05);
                let hum_drift = ((tick_count as f32 * 0.29).cos() * 0.1) + ((tick_count as f32 * 0.17).sin() * 0.05);
                let new_temp = state.temperature.value() + temp_drift;
                let new_hum = state.humidity.value() + hum_drift;
                state.temperature = Temperature::new(new_temp);
                state.humidity = Humidity::new(new_hum);

                // Battery drain: -1% per 5 minutes. Reset to 80% at 0%.
                let mut send_battery_notify = false;
                if tick_count.is_multiple_of(BATTERY_DRAIN_INTERVAL) {
                    let current = state.battery.value();
                    if current == 0 {
                        state.battery = BatteryLevel::new(BATTERY_RESET_LEVEL);
                        send_battery_notify = true;
                    } else {
                        state.battery = BatteryLevel::new(current - 1);
                        // Send notification when crossing into low threshold.
                        if current - 1 <= BATTERY_LOW_THRESHOLD && current > BATTERY_LOW_THRESHOLD {
                            send_battery_notify = true;
                        }
                    }
                }

                let temp = state.temperature;
                let hum = state.humidity;
                let battery = state.battery;
                drop(state);

                // Send sensor notification with battery appended so callers
                // can verify that battery-from-sensor-notification works.
                let sensor = SensorNotification::with_battery(temp, hum, battery);
                if tx.send((sensor_uuid, sensor.encode())).is_err() {
                    break;
                }

                // Send battery notification when low or reset.
                if send_battery_notify && tx.send((battery_uuid, vec![battery.value()])).is_err() {
                    break;
                }
            }
        });

        // Store the task handle in the connection entry.
        if let Ok(mut connections) = self.connections.try_lock() {
            if let Some(conn) = connections.get_mut(&address) {
                conn.sensor_task_handle = Some(handle);
            }
        }
    }
}

impl Default for VirtualClockTransport {
    fn default() -> Self {
        Self::new()
    }
}

#[async_trait]
impl BleTransport for VirtualClockTransport {
    async fn start_scan(&self, _filter_uuid: Uuid) -> Result<()> {
        *self.scan_index.lock().await = 0;
        Ok(())
    }

    async fn stop_scan(&self) -> Result<()> {
        Ok(())
    }

    async fn next_advertisement(&self) -> Option<AdvertisementData> {
        // First return any manually-added advertisements.
        if let Some(adv) = self.advertisements.lock().await.pop() {
            return Some(adv);
        }
        // Then return default device advertisements one by one.
        let devices = self.devices.lock().await;
        let mut index = self.scan_index.lock().await;
        let macs: Vec<MacAddress> = devices.keys().copied().collect();
        if *index >= macs.len() {
            return None;
        }
        let mac = macs[*index];
        *index += 1;
        let state_arc = devices.get(&mac)?;
        let state = state_arc.lock().await;
        Some(AdvertisementData {
            mac,
            temperature: state.temperature,
            humidity: state.humidity,
            battery: state.battery,
        })
    }

    async fn connect(&self, address: &MacAddress) -> Result<()> {
        // Create the device on-the-fly if it doesn't exist yet.
        {
            let mut devices = self.devices.lock().await;
            if !devices.contains_key(address) {
                devices.insert(*address, Arc::new(Mutex::new(VirtualDeviceState::default())));
            }
        }
        // Create per-device notification channel and connection entry.
        let (tx, rx) = mpsc::unbounded_channel();
        {
            let mut connections = self.connections.lock().await;
            connections.insert(
                *address,
                VirtualConnection {
                    notifications_tx: tx,
                    notifications_rx: Arc::new(Mutex::new(rx)),
                    sensor_task_handle: None,
                    subscribed: Vec::new(),
                },
            );
        }
        // Start periodic sensor notifications for this device.
        self.start_sensor_task(*address);
        Ok(())
    }

    async fn disconnect(&self, address: &MacAddress) -> Result<()> {
        let entry = {
            let mut connections = self.connections.lock().await;
            connections.remove(address)
        };
        if let Some(mut entry) = entry {
            if let Some(handle) = entry.sensor_task_handle.take() {
                handle.abort();
            }
        }
        Ok(())
    }

    async fn write(&self, address: &MacAddress, characteristic: CharacteristicUuid, data: &[u8]) -> Result<()> {
        match characteristic {
            CharacteristicUuid::AuthWrite => self.handle_auth_write(address, data).await,
            CharacteristicUuid::DataWrite => self.handle_data_write(address, data).await,
            _ => {
                // Ignore writes to other characteristics.
            }
        }
        Ok(())
    }

    async fn subscribe(&self, address: &MacAddress, characteristic: CharacteristicUuid) -> Result<()> {
        let mut connections = self.connections.lock().await;
        let conn = connections.get_mut(address).ok_or(TransportError::NotConnected)?;
        conn.subscribed.push(characteristic);
        Ok(())
    }

    async fn next_notification(&self, address: &MacAddress) -> Option<(Uuid, Vec<u8>)> {
        let rx_arc = self.notification_rx(address).await?;
        rx_arc.lock().await.recv().await
    }

    async fn read(&self, address: &MacAddress, characteristic: CharacteristicUuid) -> Result<Vec<u8>> {
        match characteristic {
            CharacteristicUuid::BatteryLevel => {
                let state_arc = self.device_state(address).await?;
                let state = state_arc.lock().await;
                Ok(vec![state.battery.value()])
            }
            _ => Err(TransportError::UnsupportedRead { characteristic }.into()),
        }
    }

    async fn request_mtu(&self, _address: &MacAddress, mtu: u16) -> Result<u16> {
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

#[cfg(test)]
mod tests {
    use super::*;
    use crate::Brightness;
    use crate::ClockTime;
    use crate::CommandFrame;
    use crate::DayMask;
    use crate::Language;
    use crate::RingtoneSignature;
    use crate::ScreenLightDuration;
    use crate::SensorNotification;
    use crate::TemperatureUnit;
    use crate::TimeFormat;
    use crate::Timezone;
    use crate::Volume;
    use crate::command::Command;

    #[tokio::test]
    async fn virtual_connect_disconnect() {
        let transport = VirtualClockTransport::new();
        let addr = MacAddress::parse("AA:BB:CC:DD:EE:FF").unwrap();
        assert!(!transport.is_connected(&addr));
        transport.connect(&addr).await.unwrap();
        assert!(transport.is_connected(&addr));
        transport.disconnect(&addr).await.unwrap();
        assert!(!transport.is_connected(&addr));
    }

    #[tokio::test]
    async fn virtual_auth_handshake() {
        let transport = VirtualClockTransport::new();
        let addr = MacAddress::parse("AA:BB:CC:DD:EE:FF").unwrap();
        transport.connect(&addr).await.unwrap();

        // Send Auth Init with a 16-byte token.
        let token = [0xAA; 16];
        let frame = CommandFrame::from_command(Command::AuthInit, token.to_vec());
        transport.write(&addr, CharacteristicUuid::AuthWrite, &frame.encode()).await.unwrap();

        // Expect ACK on Auth Notify.
        let (uuid, data) = transport.next_notification(&addr).await.unwrap();
        assert_eq!(uuid, CharacteristicUuid::AuthNotify.uuid());
        assert_eq!(data, vec![0x04, 0xff, 0x01, 0x00, 0x00]);

        // Send Auth Confirm.
        let frame = CommandFrame::from_command(Command::AuthConfirm, token.to_vec());
        transport.write(&addr, CharacteristicUuid::AuthWrite, &frame.encode()).await.unwrap();

        let (uuid, data) = transport.next_notification(&addr).await.unwrap();
        assert_eq!(uuid, CharacteristicUuid::AuthNotify.uuid());
        assert_eq!(data, vec![0x04, 0xff, 0x02, 0x00, 0x00]);

        let state_arc = transport.connected_state().await.unwrap();
        let state = state_arc.lock().await;
        assert!(state.authenticated);
    }

    #[tokio::test]
    async fn virtual_read_firmware() {
        let transport = VirtualClockTransport::new();
        let addr = MacAddress::parse("AA:BB:CC:DD:EE:FF").unwrap();
        transport.connect(&addr).await.unwrap();

        let frame = CommandFrame::from_command(Command::ReadFirmware, vec![]);
        transport.write(&addr, CharacteristicUuid::AuthWrite, &frame.encode()).await.unwrap();

        let (uuid, data) = transport.next_notification(&addr).await.unwrap();
        assert_eq!(uuid, CharacteristicUuid::AuthNotify.uuid());
        // Response: [length] [0x0d] [version string]
        assert_eq!(data[1], 0x0d);
        let version = String::from_utf8_lossy(&data[2..]);
        assert_eq!(version, VIRTUAL_FIRMWARE);
    }

    #[tokio::test]
    async fn virtual_read_battery() {
        let transport = VirtualClockTransport::new();
        let addr = MacAddress::parse("AA:BB:CC:DD:E0:01").unwrap();
        transport.connect(&addr).await.unwrap();
        transport.set_battery(&addr, 42).await;

        let data = transport.read(&addr, CharacteristicUuid::BatteryLevel).await.unwrap();
        assert_eq!(data, vec![42]);
    }

    #[tokio::test]
    async fn virtual_set_and_read_alarm() {
        let transport = VirtualClockTransport::new();
        let addr = MacAddress::parse("AA:BB:CC:DD:EE:FF").unwrap();
        transport.connect(&addr).await.unwrap();

        // Set alarm at slot 2: 07:30, weekdays, enabled, snooze.
        let entry = AlarmEntry::new(ClockTime::new(7, 30).unwrap(), DayMask::WEEKDAYS, true, true);
        let payload = entry.encode_set_payload(AlarmSlotIndex::new(2).unwrap());
        let frame = CommandFrame::from_command(Command::SetAlarm, payload.to_vec());
        transport.write(&addr, CharacteristicUuid::DataWrite, &frame.encode()).await.unwrap();

        // Expect ACK.
        let (uuid, data) = transport.next_notification(&addr).await.unwrap();
        assert_eq!(uuid, CharacteristicUuid::DataNotify.uuid());
        assert_eq!(data, vec![0x04, 0xff, 0x05, 0x00, 0x00]);

        // Read alarms — should get 6 packets.
        let frame = CommandFrame::from_command(Command::ReadAlarms, vec![]);
        transport.write(&addr, CharacteristicUuid::DataWrite, &frame.encode()).await.unwrap();

        let mut packets = Vec::new();
        for _ in 0..6 {
            let (uuid, data) = transport.next_notification(&addr).await.unwrap();
            assert_eq!(uuid, CharacteristicUuid::DataNotify.uuid());
            packets.push(data);
        }

        // First packet (base=0) should contain the alarm at slot 2.
        assert_eq!(packets[0][1], 0x06); // command echo
        assert_eq!(packets[0][2], 0x00); // base index
        // Slot 2 is at offset 3 + 2*5 = 13
        let slot2 = &packets[0][13..18];
        assert_eq!(slot2, &[0x01, 0x07, 0x1E, 0x3E, 0x01]);
    }

    #[tokio::test]
    async fn virtual_delete_alarm() {
        let transport = VirtualClockTransport::new();
        let addr = MacAddress::parse("AA:BB:CC:DD:EE:FF").unwrap();
        transport.connect(&addr).await.unwrap();

        // Set alarm at slot 0.
        let entry = AlarmEntry::new(ClockTime::new(6, 0).unwrap(), DayMask::EVERY_DAY, true, false);
        transport.set_alarm(&addr, AlarmSlotIndex::new(0).unwrap(), entry).await;

        // Delete alarm at slot 0.
        let payload = [0x00, 0xFF, 0xFF, 0xFF, 0xFF, 0xFF];
        let frame = CommandFrame::from_command(Command::SetAlarm, payload.to_vec());
        transport.write(&addr, CharacteristicUuid::DataWrite, &frame.encode()).await.unwrap();

        let (uuid, data) = transport.next_notification(&addr).await.unwrap();
        assert_eq!(uuid, CharacteristicUuid::DataNotify.uuid());
        assert_eq!(data, vec![0x04, 0xff, 0x05, 0x00, 0x00]);

        let state_arc = transport.connected_state().await.unwrap();
        let state = state_arc.lock().await;
        assert!(state.alarms[0].is_none());
    }

    #[tokio::test]
    async fn virtual_read_settings() {
        let transport = VirtualClockTransport::new();
        let addr = MacAddress::parse("AA:BB:CC:DD:EE:FF").unwrap();
        transport.connect(&addr).await.unwrap();

        let frame = CommandFrame::from_command(Command::ReadSettings, vec![]);
        transport.write(&addr, CharacteristicUuid::DataWrite, &frame.encode()).await.unwrap();

        let (uuid, data) = transport.next_notification(&addr).await.unwrap();
        assert_eq!(uuid, CharacteristicUuid::DataNotify.uuid());
        assert_eq!(data[1], 0x02); // command echo
        assert_eq!(data.len(), 20); // 2 header + 18 payload
    }

    #[tokio::test]
    async fn virtual_write_settings() {
        let transport = VirtualClockTransport::new();
        let addr = MacAddress::parse("AA:BB:CC:DD:EE:FF").unwrap();
        transport.connect(&addr).await.unwrap();

        let settings = DeviceSettings::new(
            Volume::new(5).unwrap(),
            TimeFormat::TwelveHour,
            TemperatureUnit::Fahrenheit,
            Language::Chinese,
            Timezone::from_hours(-5).unwrap(),
            ScreenLightDuration::new(15).unwrap(),
            Brightness::new(100).unwrap(),
            Brightness::new(20).unwrap(),
            ClockTime::new(23, 0).unwrap(),
            ClockTime::new(6, 30).unwrap(),
            false,
            true,
            RingtoneSignature::Unused,
        )
        .unwrap();

        let payload = settings.encode();
        let frame = CommandFrame::from_command(Command::SetSettings, payload.to_vec());
        transport.write(&addr, CharacteristicUuid::DataWrite, &frame.encode()).await.unwrap();

        let (uuid, data) = transport.next_notification(&addr).await.unwrap();
        assert_eq!(uuid, CharacteristicUuid::DataNotify.uuid());
        assert_eq!(data, vec![0x04, 0xff, 0x01, 0x00, 0x00]);

        // Verify settings were stored.
        let state_arc = transport.connected_state().await.unwrap();
        let state = state_arc.lock().await;
        assert_eq!(state.settings.volume().value(), 5);
        assert_eq!(state.settings.time_format(), TimeFormat::TwelveHour);
    }

    #[tokio::test]
    async fn virtual_scan_returns_advertisement() {
        let mac = MacAddress::parse("AA:BB:CC:DD:EE:FF").unwrap();
        let transport = VirtualClockTransport::with_advertisement(mac);

        let adv = transport.next_advertisement().await.unwrap();
        assert_eq!(adv.mac, mac);
    }

    #[tokio::test]
    async fn virtual_scan_returns_default_devices() {
        let transport = VirtualClockTransport::new();

        let mut found_macs = Vec::new();
        for _ in 0..5 {
            let adv = transport.next_advertisement().await.unwrap();
            found_macs.push(adv.mac);
        }
        assert_eq!(found_macs.len(), 5);
        // All MACs should be distinct.
        let unique: std::collections::HashSet<_> = found_macs.iter().collect();
        assert_eq!(unique.len(), 5);
    }

    #[tokio::test]
    async fn virtual_multi_device_independent_state() {
        let transport = VirtualClockTransport::new();
        let addr1 = MacAddress::parse("AA:BB:CC:DD:E0:01").unwrap();
        let addr2 = MacAddress::parse("AA:BB:CC:DD:E0:02").unwrap();

        // Connect to device 1 and set battery.
        transport.connect(&addr1).await.unwrap();
        transport.set_battery(&addr1, 42).await;
        transport.disconnect(&addr1).await.unwrap();

        // Connect to device 2 and set a different battery.
        transport.connect(&addr2).await.unwrap();
        transport.set_battery(&addr2, 99).await;
        transport.disconnect(&addr2).await.unwrap();

        // Reconnect to device 1 — battery should still be 42.
        transport.connect(&addr1).await.unwrap();
        let data = transport.read(&addr1, CharacteristicUuid::BatteryLevel).await.unwrap();
        assert_eq!(data, vec![42]);
        transport.disconnect(&addr1).await.unwrap();

        // Reconnect to device 2 — battery should still be 99.
        transport.connect(&addr2).await.unwrap();
        let data = transport.read(&addr2, CharacteristicUuid::BatteryLevel).await.unwrap();
        assert_eq!(data, vec![99]);
    }

    #[tokio::test]
    async fn virtual_sensor_notification() {
        let transport = VirtualClockTransport::new();
        let addr = MacAddress::parse("AA:BB:CC:DD:EE:FF").unwrap();
        transport.connect(&addr).await.unwrap();
        transport.push_sensor_notification(&addr, 23.45, 56.0);

        let (uuid, data) = transport.next_notification(&addr).await.unwrap();
        assert_eq!(uuid, CharacteristicUuid::SensorNotify.uuid());
        let sensor = SensorNotification::parse(&data).unwrap();
        assert_eq!(sensor.temperature.value(), 23.45);
        assert_eq!(sensor.humidity.value(), 56.0);
    }

    #[tokio::test]
    async fn virtual_time_sync_stores_timestamp() {
        let transport = VirtualClockTransport::new();
        let addr = MacAddress::parse("AA:BB:CC:DD:EE:FF").unwrap();
        transport.connect(&addr).await.unwrap();

        let timestamp: u32 = 1700000000;
        let frame = CommandFrame::from_command(Command::TimeSync, timestamp.to_le_bytes().to_vec());
        transport.write(&addr, CharacteristicUuid::AuthWrite, &frame.encode()).await.unwrap();

        let (_uuid, data) = transport.next_notification(&addr).await.unwrap();
        assert_eq!(data, vec![0x04, 0xff, 0x09, 0x00, 0x00]);

        let state_arc = transport.connected_state().await.unwrap();
        let state = state_arc.lock().await;
        assert_eq!(state.synced_time, Some(timestamp));
        assert!(state.synced_at.is_some());
        drop(state);

        // Device time should be approximately the synced timestamp.
        let device_time = transport.device_time(&addr).await.unwrap();
        assert!(device_time >= timestamp);
        assert!(device_time <= timestamp + 2);
    }

    #[tokio::test]
    async fn virtual_multi_connect_simultaneous() {
        let transport = VirtualClockTransport::new_differentiated();
        let addr1 = MacAddress::parse("AA:BB:CC:DD:E0:01").unwrap();
        let addr2 = MacAddress::parse("AA:BB:CC:DD:E0:02").unwrap();

        // Connect to both devices simultaneously.
        transport.connect(&addr1).await.unwrap();
        transport.connect(&addr2).await.unwrap();

        // Both should be connected.
        assert!(transport.is_connected(&addr1));
        assert!(transport.is_connected(&addr2));

        // Read battery from device 1 — should be 100.
        let data1 = transport.read(&addr1, CharacteristicUuid::BatteryLevel).await.unwrap();
        assert_eq!(data1, vec![100]);

        // Read battery from device 2 — should be 87.
        let data2 = transport.read(&addr2, CharacteristicUuid::BatteryLevel).await.unwrap();
        assert_eq!(data2, vec![87]);

        // Disconnect device 1; device 2 should still be connected.
        transport.disconnect(&addr1).await.unwrap();
        assert!(!transport.is_connected(&addr1));
        assert!(transport.is_connected(&addr2));

        // Device 2 battery should still be readable.
        let data2_again = transport.read(&addr2, CharacteristicUuid::BatteryLevel).await.unwrap();
        assert_eq!(data2_again, vec![87]);

        transport.disconnect(&addr2).await.unwrap();
    }

    #[tokio::test]
    async fn virtual_selective_disconnect() {
        let transport = VirtualClockTransport::new_differentiated();
        let addr1 = MacAddress::parse("AA:BB:CC:DD:E0:01").unwrap();
        let addr2 = MacAddress::parse("AA:BB:CC:DD:E0:03").unwrap();

        transport.connect(&addr1).await.unwrap();
        transport.connect(&addr2).await.unwrap();

        // Disconnect only device 1.
        transport.disconnect(&addr1).await.unwrap();
        assert!(!transport.is_connected(&addr1));
        assert!(transport.is_connected(&addr2));

        // Device 2 should still have its state — verify battery (65 for device 3).
        let battery = transport.read(&addr2, CharacteristicUuid::BatteryLevel).await.unwrap();
        assert_eq!(battery, vec![65]);

        transport.disconnect(&addr2).await.unwrap();
    }

    #[tokio::test]
    async fn virtual_notification_routing() {
        let transport = VirtualClockTransport::new();
        let addr1 = MacAddress::parse("AA:BB:CC:DD:E0:01").unwrap();
        let addr2 = MacAddress::parse("AA:BB:CC:DD:E0:02").unwrap();

        transport.connect(&addr1).await.unwrap();
        transport.connect(&addr2).await.unwrap();

        // Push a sensor notification on device 1's channel.
        transport.push_sensor_notification(&addr1, 27.7, 50.3);

        // Device 1 should receive it.
        let (uuid1, data1) = transport.next_notification(&addr1).await.unwrap();
        assert_eq!(uuid1, CharacteristicUuid::SensorNotify.uuid());
        let sensor1 = SensorNotification::parse(&data1).unwrap();
        assert_eq!(sensor1.temperature.value(), 27.7);
        assert_eq!(sensor1.humidity.value(), 50.3);

        // Push a different notification on device 2's channel.
        transport.push_sensor_notification(&addr2, 22.1, 45.0);

        // Device 2 should receive its own notification.
        let (uuid2, data2) = transport.next_notification(&addr2).await.unwrap();
        assert_eq!(uuid2, CharacteristicUuid::SensorNotify.uuid());
        let sensor2 = SensorNotification::parse(&data2).unwrap();
        assert_eq!(sensor2.temperature.value(), 22.1);
        assert_eq!(sensor2.humidity.value(), 45.0);

        transport.disconnect(&addr1).await.unwrap();
        transport.disconnect(&addr2).await.unwrap();
    }

    #[tokio::test]
    async fn virtual_differentiated_devices_have_distinct_values() {
        let transport = VirtualClockTransport::new_differentiated();

        let test_cases = [
            ("AA:BB:CC:DD:E0:01", 100u8, 27.7, 50.3),
            ("AA:BB:CC:DD:E0:02", 87, 22.1, 45.0),
            ("AA:BB:CC:DD:E0:03", 65, 18.5, 60.0),
            ("AA:BB:CC:DD:E0:04", 20, 15.0, 30.0),
            ("AA:BB:CC:DD:E0:05", 100, 30.2, 70.0),
        ];

        for (mac_str, expected_battery, expected_temp, expected_humidity) in test_cases {
            let addr = MacAddress::parse(mac_str).unwrap();
            transport.connect(&addr).await.unwrap();

            let battery = transport.read(&addr, CharacteristicUuid::BatteryLevel).await.unwrap();
            assert_eq!(battery[0], expected_battery, "battery mismatch for {mac_str}");

            // Push sensor notification and verify values.
            transport.push_sensor_notification(&addr, expected_temp, expected_humidity);
            let (_uuid, data) = transport.next_notification(&addr).await.unwrap();
            let sensor = SensorNotification::parse(&data).unwrap();
            assert_eq!(sensor.temperature.value(), expected_temp, "temp mismatch for {mac_str}");
            assert_eq!(sensor.humidity.value(), expected_humidity, "humidity mismatch for {mac_str}");

            transport.disconnect(&addr).await.unwrap();
        }
    }

    #[tokio::test]
    async fn virtual_scan_while_connected() {
        let transport = VirtualClockTransport::new_differentiated();
        let addr1 = MacAddress::parse("AA:BB:CC:DD:E0:01").unwrap();

        transport.connect(&addr1).await.unwrap();
        assert!(transport.is_connected(&addr1));

        // Scanning should still work while connected.
        transport.start_scan(CharacteristicUuid::AuthWrite.uuid()).await.unwrap();
        let mut found = false;
        for _ in 0..5 {
            if let Some(adv) = transport.next_advertisement().await {
                if adv.mac == addr1 {
                    found = true;
                    break;
                }
            }
        }
        assert!(found, "connected device should appear in scan results");

        // Connection should still be active after scan.
        assert!(transport.is_connected(&addr1));

        transport.disconnect(&addr1).await.unwrap();
    }
}
