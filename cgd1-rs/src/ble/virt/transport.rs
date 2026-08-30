use std::collections::HashMap;
use std::sync::Arc;
use std::sync::atomic::AtomicBool;
use std::sync::atomic::Ordering;
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
use crate::Humidity;
use crate::MacAddress;
use crate::SensorNotification;
use crate::Temperature;
use crate::ble::transport::BleTransport;
use crate::command::AlarmEntry;
use crate::command::AlarmSlotIndex;
use crate::command::Command;
use crate::command::CommandId;
use crate::command::DeviceSettings;
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
pub struct VirtualClockTransport {
    connected: AtomicBool,
    connected_mac: Arc<Mutex<Option<MacAddress>>>,
    devices: Arc<Mutex<HashMap<MacAddress, Arc<Mutex<VirtualDeviceState>>>>>,
    notifications_tx: mpsc::UnboundedSender<(Uuid, Vec<u8>)>,
    notifications_rx: Mutex<mpsc::UnboundedReceiver<(Uuid, Vec<u8>)>>,
    advertisements: Mutex<Vec<AdvertisementData>>,
    subscribed: Mutex<Vec<CharacteristicUuid>>,
    sensor_task_handle: Mutex<Option<tokio::task::JoinHandle<()>>>,
    scan_index: Mutex<usize>,
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
        let (tx, rx) = mpsc::unbounded_channel();
        Self {
            connected: AtomicBool::new(false),
            connected_mac: Arc::new(Mutex::new(None)),
            devices: Arc::new(Mutex::new(devices)),
            notifications_tx: tx,
            notifications_rx: Mutex::new(rx),
            advertisements: Mutex::new(Vec::new()),
            subscribed: Mutex::new(Vec::new()),
            sensor_task_handle: Mutex::new(None),
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

    /// Get the state of the currently connected device.
    ///
    /// Returns an error if not connected or the MAC is unknown.
    async fn connected_state(&self) -> Result<Arc<Mutex<VirtualDeviceState>>> {
        let mac = self.connected_mac.lock().await;
        let mac = mac.ok_or(TransportError::NotConnected)?;
        let devices = self.devices.lock().await;
        devices.get(&mac).cloned().ok_or(TransportError::UnknownDeviceMac { mac }.into())
    }

    /// Set the battery level of the currently connected device.
    pub async fn set_battery(&self, level: u8) {
        if let Ok(state_arc) = self.connected_state().await {
            let mut state = state_arc.lock().await;
            state.battery = BatteryLevel::new(level);
        }
    }

    /// Set the sensor values of the currently connected device.
    pub async fn set_sensor_values(&self, temperature: f32, humidity: f32) {
        if let Ok(state_arc) = self.connected_state().await {
            let mut state = state_arc.lock().await;
            state.temperature = Temperature::new(temperature);
            state.humidity = Humidity::new(humidity);
        }
    }

    /// Set an alarm in the currently connected device's state directly.
    pub async fn set_alarm(&self, slot: AlarmSlotIndex, entry: AlarmEntry) {
        if let Ok(state_arc) = self.connected_state().await {
            let mut state = state_arc.lock().await;
            state.alarms[slot.value() as usize] = Some(entry);
        }
    }

    /// Get the current device time as a Unix timestamp.
    ///
    /// Returns `synced_time + elapsed_seconds` since the last Time Sync.
    /// Returns `None` if no Time Sync has been performed.
    pub async fn device_time(&self) -> Option<u32> {
        let state_arc = self.connected_state().await.ok()?;
        let state = state_arc.lock().await;
        let synced_time = state.synced_time?;
        let synced_at = state.synced_at?;
        let elapsed = synced_at.elapsed().as_secs() as u32;
        Some(synced_time + elapsed)
    }

    /// Push a sensor notification manually (for testing event subscription).
    pub fn push_sensor_notification(&self, temperature: f32, humidity: f32) {
        let sensor = SensorNotification::new(Temperature::new(temperature), Humidity::new(humidity));
        self.push_notification(CharacteristicUuid::SensorNotify.uuid(), sensor.encode().to_vec());
    }

    /// Push a battery notification manually (for testing event subscription).
    pub fn push_battery_notification(&self, level: u8) {
        self.push_notification(CharacteristicUuid::BatteryLevel.uuid(), vec![level]);
    }

    /// Push a raw notification onto the notification channel.
    fn push_notification(&self, uuid: Uuid, data: Vec<u8>) {
        let _ = self.notifications_tx.send((uuid, data));
    }

    /// Send an ACK frame on the given notify characteristic.
    fn send_ack_on(&self, notify_char: CharacteristicUuid, command_byte: u8, status: u8, payload: u8) {
        let ack = vec![0x04, 0xff, command_byte, status, payload];
        self.push_notification(notify_char.uuid(), ack);
    }

    /// Send a success ACK on the given notify characteristic.
    fn send_success_ack_on(&self, notify_char: CharacteristicUuid, command_byte: u8) {
        self.send_ack_on(notify_char, command_byte, 0x00, 0x00);
    }

    /// Send a data notification (non-ACK) on the Data Notify characteristic.
    fn send_data_notification(&self, data: Vec<u8>) {
        self.push_notification(CharacteristicUuid::DataNotify.uuid(), data);
    }

    /// Send a data notification on the Auth Notify characteristic.
    fn send_auth_notification(&self, data: Vec<u8>) {
        self.push_notification(CharacteristicUuid::AuthNotify.uuid(), data);
    }

    /// Handle a command frame written to Auth Write.
    async fn handle_auth_write(&self, data: &[u8]) {
        if data.len() < 2 {
            return;
        }
        let command_id = CommandId::new(data[1]);
        let payload = &data[2..];
        let notify = CharacteristicUuid::AuthNotify;

        let command = match Command::from_id_for_characteristic(command_id, CharacteristicUuid::AuthWrite) {
            Some(cmd) => cmd,
            None => {
                self.send_ack_on(notify, command_id.value(), 0x01, 0x00);
                return;
            }
        };

        match command {
            Command::AuthInit => {
                // Auth Init — accept any 16-byte token.
                if let Ok(state_arc) = self.connected_state().await {
                    let mut state = state_arc.lock().await;
                    if payload.len() >= 16 {
                        let mut token = [0u8; 16];
                        token.copy_from_slice(&payload[..16]);
                        state.token = Some(token);
                    }
                }
                self.send_success_ack_on(notify, command_id.value());
            }
            Command::AuthConfirm => {
                // Auth Confirm — accept and mark as authenticated.
                if let Ok(state_arc) = self.connected_state().await {
                    let mut state = state_arc.lock().await;
                    state.authenticated = true;
                }
                self.send_success_ack_on(notify, command_id.value());
            }
            Command::TimeSync => {
                // Time Sync — store the timestamp and the instant it was set.
                if payload.len() >= 4 {
                    let timestamp = u32::from_le_bytes([payload[0], payload[1], payload[2], payload[3]]);
                    if let Ok(state_arc) = self.connected_state().await {
                        let mut state = state_arc.lock().await;
                        state.synced_time = Some(timestamp);
                        state.synced_at = Some(Instant::now());
                    }
                    self.send_success_ack_on(notify, command_id.value());
                } else {
                    self.send_ack_on(notify, command_id.value(), 0x01, 0x00);
                }
            }
            Command::ReadFirmware => {
                // Read Firmware — respond with version string on Auth Notify.
                let version_bytes = VIRTUAL_FIRMWARE.as_bytes();
                let mut response = Vec::with_capacity(2 + version_bytes.len());
                response.push((1 + version_bytes.len()) as u8);
                response.push(command_id.value());
                response.extend_from_slice(version_bytes);
                self.send_auth_notification(response);
            }
            _ => {
                self.send_ack_on(notify, command_id.value(), 0x01, 0x00);
            }
        }
    }

    /// Handle a command frame written to Data Write.
    async fn handle_data_write(&self, data: &[u8]) {
        if data.len() < 2 {
            return;
        }
        let command_id = CommandId::new(data[1]);
        let payload = &data[2..];
        let notify = CharacteristicUuid::DataNotify;

        let command = match Command::from_id_for_characteristic(command_id, CharacteristicUuid::DataWrite) {
            Some(cmd) => cmd,
            None => {
                self.send_ack_on(notify, command_id.value(), 0x01, 0x00);
                return;
            }
        };

        match command {
            Command::SetSettings => {
                // Set Settings — decode and store.
                if payload.len() >= 18 {
                    if let Ok(settings) = DeviceSettings::decode(payload) {
                        if let Ok(state_arc) = self.connected_state().await {
                            let mut state = state_arc.lock().await;
                            state.settings = settings;
                        }
                        self.send_success_ack_on(notify, command_id.value());
                    } else {
                        self.send_ack_on(notify, command_id.value(), 0x01, 0x00);
                    }
                } else {
                    self.send_ack_on(notify, command_id.value(), 0x01, 0x00);
                }
            }
            Command::ReadSettings => {
                // Read Settings — respond with encoded settings on Data Notify.
                if let Ok(state_arc) = self.connected_state().await {
                    let state = state_arc.lock().await;
                    let encoded = state.settings.encode();
                    drop(state);
                    let mut response = Vec::with_capacity(20);
                    response.push(0x13);
                    response.push(command_id.value());
                    response.extend_from_slice(&encoded);
                    self.send_data_notification(response);
                }
            }
            Command::SetBrightness => {
                // Set Brightness — accept, no-op.
                self.send_success_ack_on(notify, command_id.value());
            }
            Command::PreviewRingtone => {
                // Preview Ringtone — accept, no-op.
                self.send_success_ack_on(notify, command_id.value());
            }
            Command::SetAlarm => {
                // Set/Delete Alarm.
                if payload.len() >= 6 {
                    let slot_byte = payload[0];
                    let entry_bytes = &payload[1..6];

                    if entry_bytes.iter().all(|&b| b == 0xFF) {
                        if let Ok(slot) = AlarmSlotIndex::new(slot_byte) {
                            if let Ok(state_arc) = self.connected_state().await {
                                let mut state = state_arc.lock().await;
                                state.alarms[slot.value() as usize] = None;
                            }
                            self.send_success_ack_on(notify, command_id.value());
                        } else {
                            self.send_ack_on(notify, command_id.value(), 0x01, 0x00);
                        }
                    } else if let Ok(Some(entry)) = AlarmEntry::decode(entry_bytes) {
                        if let Ok(slot) = AlarmSlotIndex::new(slot_byte) {
                            if let Ok(state_arc) = self.connected_state().await {
                                let mut state = state_arc.lock().await;
                                state.alarms[slot.value() as usize] = Some(entry);
                            }
                            self.send_success_ack_on(notify, command_id.value());
                        } else {
                            self.send_ack_on(notify, command_id.value(), 0x01, 0x00);
                        }
                    } else {
                        self.send_ack_on(notify, command_id.value(), 0x01, 0x00);
                    }
                } else {
                    self.send_ack_on(notify, command_id.value(), 0x01, 0x00);
                }
            }
            Command::ReadAlarms => {
                // Read Alarms — send 6 packets with 3 entries each.
                if let Ok(state_arc) = self.connected_state().await {
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
                        self.send_data_notification(response);
                    }
                }
            }
            Command::AudioInit => {
                // Audio Init — accept, start upload.
                if payload.len() >= 7 {
                    let total_size = u32::from_le_bytes([payload[0], payload[1], payload[2], 0]) as usize;
                    if let Ok(state_arc) = self.connected_state().await {
                        let mut state = state_arc.lock().await;
                        state.audio_upload_active = true;
                        state.audio_upload_total = total_size;
                        state.audio_upload_received = 0;
                        state.audio_block_packets = 0;
                    }
                    self.send_ack_on(notify, command_id.value(), 0x00, 0x00);
                } else {
                    self.send_ack_on(notify, command_id.value(), 0x01, 0x00);
                }
            }
            Command::AudioData => {
                // Audio Data Packet — accept, track progress.
                if let Ok(state_arc) = self.connected_state().await {
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
                            self.send_success_ack_on(notify, command_id.value());
                        }
                    } else {
                        drop(state);
                        self.send_ack_on(notify, command_id.value(), 0x01, 0x00);
                    }
                }
            }
            _ => {
                self.send_ack_on(notify, command_id.value(), 0x01, 0x00);
            }
        }
    }

    /// Start a background task that periodically sends sensor notifications
    /// with slight temperature/humidity drift and battery drain.
    fn start_sensor_task(&self) {
        let tx = self.notifications_tx.clone();
        let sensor_uuid = CharacteristicUuid::SensorNotify.uuid();
        let battery_uuid = CharacteristicUuid::BatteryLevel.uuid();
        let devices = self.devices.clone();
        let connected_mac = self.connected_mac.clone();

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
                let mac_guard = connected_mac.lock().await;
                let mac = match *mac_guard {
                    Some(m) => m,
                    None => continue,
                };
                drop(mac_guard);
                let devices_map = devices.lock().await;
                let state_arc = match devices_map.get(&mac) {
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

                // Send sensor notification.
                let sensor = SensorNotification::new(temp, hum);
                if tx.send((sensor_uuid, sensor.encode().to_vec())).is_err() {
                    break;
                }

                // Send battery notification when low or reset.
                if send_battery_notify && tx.send((battery_uuid, vec![battery.value()])).is_err() {
                    break;
                }
            }
        });

        if let Ok(mut handle_guard) = self.sensor_task_handle.try_lock() {
            *handle_guard = Some(handle);
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
        {
            let mut mac = self.connected_mac.lock().await;
            *mac = Some(*address);
        }
        self.connected.store(true, Ordering::SeqCst);
        // Start periodic sensor notifications.
        self.start_sensor_task();
        Ok(())
    }

    async fn disconnect(&self) -> Result<()> {
        self.connected.store(false, Ordering::SeqCst);
        {
            let mut mac = self.connected_mac.lock().await;
            *mac = None;
        }
        // Mark the previously connected device as unauthenticated.
        // (We can't easily access it here without tracking the old MAC,
        // but the sensor task will stop on its own when mac is None.)
        // Stop the sensor task.
        #[allow(clippy::collapsible_if)]
        if let Ok(mut handle_guard) = self.sensor_task_handle.try_lock() {
            if let Some(handle) = handle_guard.take() {
                handle.abort();
            }
        }
        Ok(())
    }

    async fn write(&self, characteristic: CharacteristicUuid, data: &[u8]) -> Result<()> {
        match characteristic {
            CharacteristicUuid::AuthWrite => self.handle_auth_write(data).await,
            CharacteristicUuid::DataWrite => self.handle_data_write(data).await,
            _ => {
                // Ignore writes to other characteristics.
            }
        }
        Ok(())
    }

    async fn subscribe(&self, characteristic: CharacteristicUuid) -> Result<()> {
        self.subscribed.lock().await.push(characteristic);
        Ok(())
    }

    async fn next_notification(&self) -> Option<(Uuid, Vec<u8>)> {
        self.notifications_rx.lock().await.recv().await
    }

    async fn read(&self, characteristic: CharacteristicUuid) -> Result<Vec<u8>> {
        match characteristic {
            CharacteristicUuid::BatteryLevel => {
                let state_arc = self.connected_state().await?;
                let state = state_arc.lock().await;
                Ok(vec![state.battery.value()])
            }
            _ => Err(TransportError::UnsupportedRead { characteristic }.into()),
        }
    }

    async fn request_mtu(&self, mtu: u16) -> Result<u16> {
        Ok(mtu)
    }

    fn is_connected(&self) -> bool {
        self.connected.load(Ordering::SeqCst)
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
        assert!(!transport.is_connected());
        transport.connect(&addr).await.unwrap();
        assert!(transport.is_connected());
        transport.disconnect().await.unwrap();
        assert!(!transport.is_connected());
    }

    #[tokio::test]
    async fn virtual_auth_handshake() {
        let transport = VirtualClockTransport::new();
        let addr = MacAddress::parse("AA:BB:CC:DD:EE:FF").unwrap();
        transport.connect(&addr).await.unwrap();

        // Send Auth Init with a 16-byte token.
        let token = [0xAA; 16];
        let frame = CommandFrame::from_command(Command::AuthInit, token.to_vec());
        transport.write(CharacteristicUuid::AuthWrite, &frame.encode()).await.unwrap();

        // Expect ACK on Auth Notify.
        let (uuid, data) = transport.next_notification().await.unwrap();
        assert_eq!(uuid, CharacteristicUuid::AuthNotify.uuid());
        assert_eq!(data, vec![0x04, 0xff, 0x01, 0x00, 0x00]);

        // Send Auth Confirm.
        let frame = CommandFrame::from_command(Command::AuthConfirm, token.to_vec());
        transport.write(CharacteristicUuid::AuthWrite, &frame.encode()).await.unwrap();

        let (uuid, data) = transport.next_notification().await.unwrap();
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
        transport.write(CharacteristicUuid::AuthWrite, &frame.encode()).await.unwrap();

        let (uuid, data) = transport.next_notification().await.unwrap();
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
        transport.set_battery(42).await;

        let data = transport.read(CharacteristicUuid::BatteryLevel).await.unwrap();
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
        transport.write(CharacteristicUuid::DataWrite, &frame.encode()).await.unwrap();

        // Expect ACK.
        let (uuid, data) = transport.next_notification().await.unwrap();
        assert_eq!(uuid, CharacteristicUuid::DataNotify.uuid());
        assert_eq!(data, vec![0x04, 0xff, 0x05, 0x00, 0x00]);

        // Read alarms — should get 6 packets.
        let frame = CommandFrame::from_command(Command::ReadAlarms, vec![]);
        transport.write(CharacteristicUuid::DataWrite, &frame.encode()).await.unwrap();

        let mut packets = Vec::new();
        for _ in 0..6 {
            let (uuid, data) = transport.next_notification().await.unwrap();
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
        transport.set_alarm(AlarmSlotIndex::new(0).unwrap(), entry).await;

        // Delete alarm at slot 0.
        let payload = [0x00, 0xFF, 0xFF, 0xFF, 0xFF, 0xFF];
        let frame = CommandFrame::from_command(Command::SetAlarm, payload.to_vec());
        transport.write(CharacteristicUuid::DataWrite, &frame.encode()).await.unwrap();

        let (uuid, data) = transport.next_notification().await.unwrap();
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
        transport.write(CharacteristicUuid::DataWrite, &frame.encode()).await.unwrap();

        let (uuid, data) = transport.next_notification().await.unwrap();
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
        transport.write(CharacteristicUuid::DataWrite, &frame.encode()).await.unwrap();

        let (uuid, data) = transport.next_notification().await.unwrap();
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
        transport.set_battery(42).await;
        transport.disconnect().await.unwrap();

        // Connect to device 2 and set a different battery.
        transport.connect(&addr2).await.unwrap();
        transport.set_battery(99).await;
        transport.disconnect().await.unwrap();

        // Reconnect to device 1 — battery should still be 42.
        transport.connect(&addr1).await.unwrap();
        let data = transport.read(CharacteristicUuid::BatteryLevel).await.unwrap();
        assert_eq!(data, vec![42]);
        transport.disconnect().await.unwrap();

        // Reconnect to device 2 — battery should still be 99.
        transport.connect(&addr2).await.unwrap();
        let data = transport.read(CharacteristicUuid::BatteryLevel).await.unwrap();
        assert_eq!(data, vec![99]);
    }

    #[tokio::test]
    async fn virtual_sensor_notification() {
        let transport = VirtualClockTransport::new();
        transport.push_sensor_notification(23.45, 56.0);

        let (uuid, data) = transport.next_notification().await.unwrap();
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
        transport.write(CharacteristicUuid::AuthWrite, &frame.encode()).await.unwrap();

        let (_uuid, data) = transport.next_notification().await.unwrap();
        assert_eq!(data, vec![0x04, 0xff, 0x09, 0x00, 0x00]);

        let state_arc = transport.connected_state().await.unwrap();
        let state = state_arc.lock().await;
        assert_eq!(state.synced_time, Some(timestamp));
        assert!(state.synced_at.is_some());
        drop(state);

        // Device time should be approximately the synced timestamp.
        let device_time = transport.device_time().await.unwrap();
        assert!(device_time >= timestamp);
        assert!(device_time <= timestamp + 2);
    }
}
