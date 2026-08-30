use std::collections::HashMap;
use std::collections::VecDeque;
use std::sync::Arc;
use std::sync::atomic::AtomicBool;
use std::sync::atomic::Ordering;
use std::time::Duration;
use std::time::SystemTime;
use std::time::UNIX_EPOCH;

use tokio::sync::Mutex;
use tokio::sync::broadcast;
use tokio::sync::mpsc;
use tokio::sync::oneshot;
use tokio::time::timeout;
use tracing::debug;
use tracing::info;
use tracing::warn;

use crate::BleTransport;
use crate::CharacteristicUuid;
use crate::SensorNotification;
use crate::command::Ack;
use crate::command::AckStatus;
use crate::command::AlarmEntry;
use crate::command::AlarmSlot;
use crate::command::AlarmSlotIndex;
use crate::command::Brightness;
use crate::command::Command;
use crate::command::CommandId;
use crate::command::DeviceSettings;
use crate::command::Volume;
use crate::error::AuthFailedError;
use crate::error::ClockError;
use crate::error::Result;
use crate::error::TransportError;
use crate::event::ClockEvent;
use crate::token::AuthToken;
use crate::token::TokenStore;
use crate::types::BatteryLevel;
use crate::types::MacAddress;

/// Response timeout in seconds.
const RESPONSE_TIMEOUT_SECS: u64 = 10;

/// Extended ACK timeout for audio data packets (seconds).
///
/// Regular commands use `RESPONSE_TIMEOUT_SECS` (10s), but audio uploads
/// involve many sequential packets over a potentially slow BLE link.
const AUDIO_ACK_TIMEOUT_SECS: u64 = 30;

/// Audio data packet payload size (bytes).
const AUDIO_PACKET_PAYLOAD_SIZE: usize = 128;

/// Number of audio packets per block before an ACK is expected.
const AUDIO_PACKETS_PER_BLOCK: usize = 4;

/// Maximum audio data size (~12 seconds at 8 kHz 8-bit mono).
const AUDIO_MAX_SIZE: usize = 98_304;

/// Validate that audio data meets the CGD1 format requirements.
///
/// The audio must be 8-bit unsigned PCM at 8 kHz, mono. The caller is
/// responsible for providing correctly formatted audio; this function only
/// checks size constraints.
pub fn validate_audio(audio: &[u8]) -> Result<()> {
    if audio.is_empty() {
        return Err(ClockError::InvalidSettings("audio data is empty".into()));
    }
    if audio.len() > AUDIO_MAX_SIZE {
        return Err(ClockError::InvalidSettings(format!("audio too large: {} bytes (max {})", audio.len(), AUDIO_MAX_SIZE)));
    }
    Ok(())
}

/// Shared state for matching ACKs to pending requests.
///
/// Uses a `VecDeque` per command byte so that multiple concurrent requests
/// with the same command byte are queued rather than overwriting each other.
/// The notification task pops the front sender when an ACK arrives.
type PendingMap = Arc<Mutex<HashMap<CommandId, VecDeque<oneshot::Sender<Result<Ack>>>>>>;

/// Register a pending request for a given command byte.
///
/// Pushes the oneshot sender onto the queue for that command. When the
/// notification task receives an ACK with a matching command byte, it pops
/// the front sender and delivers the result.
async fn register_pending(pending: &PendingMap, command: CommandId, sender: oneshot::Sender<Result<Ack>>) {
    let mut map = pending.lock().await;
    map.entry(command).or_default().push_back(sender);
}

/// Pop the next live pending sender for a given command byte.
///
/// Called by the notification task when an ACK arrives. Skips senders whose
/// receivers have been dropped (e.g., due to timeout). Returns `None` if
/// no live request is pending for that command.
async fn pop_pending_alive(pending: &PendingMap, command: CommandId) -> Option<oneshot::Sender<Result<Ack>>> {
    let mut map = pending.lock().await;
    let queue = map.get_mut(&command)?;
    while let Some(sender) = queue.pop_front() {
        if !sender.is_closed() {
            return Some(sender);
        }
    }
    None
}

/// Handle to a connected Qingping CGD1 alarm clock.
///
/// All command methods that send a frame and wait for an ACK are serialized
/// via `command_mutex` to prevent race conditions when multiple callers
/// invoke the same command byte concurrently.
///
/// Stores the last successful auth token so that reconnection can
/// automatically re-authenticate without caller intervention.
#[derive(Clone)]
pub struct ClockDevice {
    transport: Arc<dyn BleTransport>,
    address: MacAddress,
    event_sender: broadcast::Sender<ClockEvent>,
    /// Serializes write operations to prevent concurrent same-command races.
    command_mutex: Arc<Mutex<()>>,
    /// Last successful auth token, used for automatic re-auth on reconnect.
    auth_token: Arc<Mutex<Option<AuthToken>>>,
    /// Whether the device is currently authenticated.
    is_authenticated: Arc<AtomicBool>,
    /// Pending request-response channels keyed by command byte.
    pending: PendingMap,
    /// Pending channel for non-ACK data notifications (e.g. firmware, alarms).
    /// Uses mpsc to support multi-packet responses.
    pending_data_response: Arc<Mutex<Option<mpsc::Sender<Vec<u8>>>>>,
    /// Optional token store for persisting auth tokens after successful privileged commands.
    token_store: Arc<Mutex<Option<Arc<dyn TokenStore>>>>,
}

impl ClockDevice {
    /// Create a new device handle with the given transport and address.
    ///
    /// This does not connect or subscribe — use [`ClockManager::connect`]
    /// for the full connection lifecycle.
    pub fn new(transport: Arc<dyn BleTransport>, address: MacAddress) -> Self {
        let (event_sender, _) = broadcast::channel(64);
        Self {
            transport,
            address,
            event_sender,
            command_mutex: Arc::new(Mutex::new(())),
            auth_token: Arc::new(Mutex::new(None)),
            is_authenticated: Arc::new(AtomicBool::new(false)),
            pending: Arc::new(Mutex::new(HashMap::new())),
            pending_data_response: Arc::new(Mutex::new(None)),
            token_store: Arc::new(Mutex::new(None)),
        }
    }

    /// Subscribe to device events (sensor updates, battery, disconnections).
    pub fn subscribe(&self) -> broadcast::Receiver<ClockEvent> {
        self.event_sender.subscribe()
    }

    /// Get the device MAC address.
    pub fn address(&self) -> &MacAddress {
        &self.address
    }

    /// Whether the device is currently authenticated.
    pub fn is_authenticated(&self) -> bool {
        self.is_authenticated.load(Ordering::SeqCst)
    }

    /// Set the token store used to persist auth tokens.
    ///
    /// When set, the token is saved after the first privileged command
    /// (e.g., `sync_time`) succeeds, confirming the device accepted it.
    pub async fn set_token_store(&self, store: Arc<dyn TokenStore>) {
        let mut guard = self.token_store.lock().await;
        *guard = Some(store);
    }

    /// Spawn the background notification task for this device.
    ///
    /// The task listens for BLE notifications, dispatches ACKs to pending
    /// request-response channels, broadcasts sensor/battery events, and
    /// attempts reconnection on disconnect.
    pub fn spawn_notification_task(&self) {
        let transport = self.transport.clone();
        let event_sender = self.event_sender.clone();
        let pending = self.pending.clone();
        let is_authenticated = self.is_authenticated.clone();
        let auth_token = self.auth_token.clone();
        let address = self.address;
        let command_mutex = self.command_mutex.clone();
        let pending_data_response = self.pending_data_response.clone();
        let token_store = self.token_store.clone();

        tokio::spawn(async move {
            notification_task(
                transport,
                event_sender,
                pending,
                is_authenticated,
                auth_token,
                address,
                command_mutex,
                pending_data_response,
                token_store,
            )
            .await
        });
    }

    /// Wait for an ACK with the given command byte.
    ///
    /// Registers a pending request and waits up to `RESPONSE_TIMEOUT_SECS`
    /// for the notification task to deliver the ACK.
    async fn wait_for_ack(&self, command: Command) -> Result<Ack> {
        let command_id = command.command_id();
        let (sender, receiver) = oneshot::channel();
        register_pending(&self.pending, command_id, sender).await;

        match timeout(Duration::from_secs(RESPONSE_TIMEOUT_SECS), receiver).await {
            Ok(Ok(ack)) => ack,
            Ok(Err(_)) => Err(TransportError::RequestCanceled.into()),
            Err(_) => Err(ClockError::Timeout),
        }
    }

    /// Wait for an ACK with a custom timeout duration.
    ///
    /// Used by audio upload where per-packet ACKs may take longer than
    /// the default timeout.
    async fn wait_for_ack_with_timeout(&self, command: Command, timeout_secs: u64) -> Result<Ack> {
        let command_id = command.command_id();
        let (sender, receiver) = oneshot::channel();
        register_pending(&self.pending, command_id, sender).await;

        match timeout(Duration::from_secs(timeout_secs), receiver).await {
            Ok(Ok(ack)) => ack,
            Ok(Err(_)) => Err(TransportError::RequestCanceled.into()),
            Err(_) => Err(ClockError::Timeout),
        }
    }

    /// Authenticate with the device using a 16-byte token.
    ///
    /// Performs the two-step handshake:
    /// 1. Send Auth Init: `11 01 [Token 16B]` to Auth Write.
    /// 2. Wait for ACK on Auth Notify: `04 ff 01 00 [Payload]`.
    /// 3. Send Auth Confirm: `11 02 [Token 16B]` to Auth Write.
    /// 4. Wait for final ACK: `04 ff 02 00 00`.
    pub async fn authenticate(&self, token: &AuthToken) -> Result<()> {
        let _guard = self.command_mutex.lock().await;

        // Step 1: Auth Init
        self.transport.write_frame(Command::AuthInit, token.payload()).await?;

        let ack = self.wait_for_ack(Command::AuthInit).await?;
        if let AckStatus::Failure(code) = ack.status {
            return Err(ClockError::AuthFailed(AuthFailedError {
                reason: format!("init status: {code:#04x}"),
                is_new_token: false,
                token_path: None,
            }));
        }

        // Step 2: Auth Confirm
        self.transport.write_frame(Command::AuthConfirm, token.payload()).await?;

        let ack = self.wait_for_ack(Command::AuthConfirm).await?;
        if let AckStatus::Failure(code) = ack.status {
            return Err(ClockError::AuthFailed(AuthFailedError {
                reason: format!("confirm status: {code:#04x}"),
                is_new_token: false,
                token_path: None,
            }));
        }

        // Store the token for automatic re-auth on reconnect.
        {
            let mut token_guard = self.auth_token.lock().await;
            *token_guard = Some(token.clone());
        }
        self.is_authenticated.store(true, Ordering::SeqCst);

        Ok(())
    }

    /// Synchronize the device clock to the given Unix timestamp.
    ///
    /// Sends: `05 09 [Timestamp 4B LE]` to Auth Write.
    /// Expects ACK: `04 ff 09 00 00`.
    ///
    /// This is the first privileged command after authentication. If the
    /// token was rejected, the device will drop the connection here.
    pub async fn sync_time(&self, timestamp: u32) -> Result<()> {
        let _guard = self.command_mutex.lock().await;

        let payload = timestamp.to_le_bytes();
        self.transport.write_frame(Command::TimeSync, &payload).await?;

        let ack = self.wait_for_ack(Command::TimeSync).await?;
        if let AckStatus::Failure(_) = ack.status {
            return Err(ClockError::CommandRejected {
                command: 0x09,
                status: ack.status,
            });
        }

        // Token is now confirmed — persist it if a token store is configured.
        let store = self.token_store.lock().await;
        if let Some(ref store) = *store {
            let token = self.auth_token.lock().await;
            if let Some(ref token) = *token {
                store.save(&self.address, token)?;
            }
        }

        Ok(())
    }

    /// Synchronize the device clock to the current system time.
    pub async fn sync_time_now(&self) -> Result<()> {
        let now = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .map_err(|e| ClockError::from(TransportError::SystemTime(e.to_string())))?;
        self.sync_time(now.as_secs() as u32).await
    }

    /// Read the firmware version string from the device.
    ///
    /// Sends: `01 0d` to Auth Write.
    /// Expects response on Auth Notify: `0b [Byte] [ASCII String]`.
    pub async fn read_firmware(&self) -> Result<String> {
        let _guard = self.command_mutex.lock().await;

        // Set up a pending mpsc channel for non-ACK data notifications.
        let (sender, mut receiver) = mpsc::channel(16);
        {
            let mut pending = self.pending_data_response.lock().await;
            *pending = Some(sender);
        }

        self.transport.write_frame(Command::ReadFirmware, &[]).await?;

        let response = match timeout(Duration::from_secs(RESPONSE_TIMEOUT_SECS), receiver.recv()).await {
            Ok(Some(data)) => data,
            Ok(None) => return Err(TransportError::ResponseCanceled { context: "firmware".into() }.into()),
            Err(_) => {
                let mut pending = self.pending_data_response.lock().await;
                *pending = None;
                return Err(ClockError::Timeout);
            }
        };

        // Clean up the pending channel.
        {
            let mut pending = self.pending_data_response.lock().await;
            *pending = None;
        }

        // Parse: skip length byte, skip one byte, rest is ASCII string.
        if response.len() < 2 {
            return Err(ClockError::Parse("firmware response too short".into()));
        }
        let version = String::from_utf8_lossy(&response[2..]).to_string();
        Ok(version)
    }

    /// Set or modify an alarm at the given slot index.
    ///
    /// Sends: `07 05 [ID] [Enabled] [HH] [MM] [Days] [Snooze]` to Data Write.
    /// Expects ACK: `04 ff 05 00 00`.
    pub async fn set_alarm(&self, alarm: &AlarmEntry, slot: AlarmSlotIndex) -> Result<()> {
        let _guard = self.command_mutex.lock().await;

        let payload = alarm.encode_set_payload(slot);
        self.transport.write_frame(Command::SetAlarm, &payload).await?;

        let ack = self.wait_for_ack(Command::SetAlarm).await?;
        if let AckStatus::Failure(_) = ack.status {
            return Err(ClockError::CommandRejected {
                command: 0x05,
                status: ack.status,
            });
        }

        Ok(())
    }

    /// Delete an alarm at the given slot index.
    ///
    /// Sends: `07 05 [ID] FF FF FF FF FF` to Data Write.
    /// This overwrites the slot with 0xFF values, marking it as empty.
    pub async fn delete_alarm(&self, slot: AlarmSlotIndex) -> Result<()> {
        let _guard = self.command_mutex.lock().await;

        let payload = [slot.value(), 0xFF, 0xFF, 0xFF, 0xFF, 0xFF];
        self.transport.write_frame(Command::SetAlarm, &payload).await?;

        let ack = self.wait_for_ack(Command::SetAlarm).await?;
        if let AckStatus::Failure(_) = ack.status {
            return Err(ClockError::CommandRejected {
                command: 0x05,
                status: ack.status,
            });
        }

        Ok(())
    }

    /// Read all alarm slots from the device.
    ///
    /// Sends: `01 06` to Data Write.
    /// Expects multiple response packets on Data Notify, each containing
    /// up to 3 alarm entries (5 bytes each). The device sends 6 packets
    /// for all 16 slots.
    pub async fn read_alarms(&self) -> Result<Vec<AlarmSlot>> {
        let _guard = self.command_mutex.lock().await;

        // Set up a pending mpsc channel for multi-packet data notifications.
        let (sender, mut receiver) = mpsc::channel(16);
        {
            let mut pending = self.pending_data_response.lock().await;
            *pending = Some(sender);
        }

        self.transport.write_frame(Command::ReadAlarms, &[]).await?;

        // Collect all data packets within the timeout period.
        // The device sends 6 packets for 16 slots (3 per packet).
        let mut all_data = Vec::new();
        let deadline = Duration::from_secs(RESPONSE_TIMEOUT_SECS);

        loop {
            match timeout(deadline, receiver.recv()).await {
                Ok(Some(data)) => {
                    all_data.push(data);
                    // If we've received 6 packets, we have all slots.
                    if all_data.len() >= 6 {
                        break;
                    }
                }
                Ok(None) => break,
                Err(_) => break,
            }
        }

        // Clean up the pending channel.
        {
            let mut pending = self.pending_data_response.lock().await;
            *pending = None;
        }

        if all_data.is_empty() {
            return Err(ClockError::Timeout);
        }

        // Parse all received packets.
        // Each packet: [Length] [0x06] [BaseIndex] [Entry1 5B] [Entry2 5B] [Entry3 5B]
        let mut slots = Vec::new();
        for packet in &all_data {
            if packet.len() < 3 {
                continue;
            }
            // Skip length byte (packet[0]) and command byte (packet[1]).
            // packet[2] is the base index (slot index of the first entry in this packet).
            let base_index = packet[2] as usize;
            let entries_data = &packet[3..];

            for (i, chunk) in entries_data.chunks(5).enumerate() {
                if chunk.len() < 5 {
                    break;
                }
                let raw_slot = (base_index + i) as u8;
                let Ok(index) = AlarmSlotIndex::new(raw_slot) else {
                    continue;
                };
                if let Some(entry) = AlarmEntry::decode(chunk)? {
                    slots.push(AlarmSlot { index, entry });
                }
            }
        }

        Ok(slots)
    }

    /// Read device settings.
    ///
    /// Sends: `01 02` to Data Write.
    /// Expects response on Data Notify: `13 02 [18 bytes payload]`.
    pub async fn read_settings(&self) -> Result<DeviceSettings> {
        let _guard = self.command_mutex.lock().await;

        let (sender, mut receiver) = mpsc::channel(16);
        {
            let mut pending = self.pending_data_response.lock().await;
            *pending = Some(sender);
        }

        self.transport.write_frame(Command::ReadSettings, &[]).await?;

        let response = match timeout(Duration::from_secs(RESPONSE_TIMEOUT_SECS), receiver.recv()).await {
            Ok(Some(data)) => data,
            Ok(None) => return Err(TransportError::ResponseCanceled { context: "settings".into() }.into()),
            Err(_) => {
                let mut pending = self.pending_data_response.lock().await;
                *pending = None;
                return Err(ClockError::Timeout);
            }
        };

        {
            let mut pending = self.pending_data_response.lock().await;
            *pending = None;
        }

        if response.len() < 2 {
            return Err(ClockError::Parse("settings response too short".into()));
        }
        DeviceSettings::decode(&response[2..])
    }

    /// Write device settings.
    ///
    /// Sends: `13 01 [18 bytes payload]` to Data Write.
    /// Expects ACK: `04 ff 01 00 00`.
    pub async fn write_settings(&self, settings: &DeviceSettings) -> Result<()> {
        let _guard = self.command_mutex.lock().await;

        let payload = settings.encode();
        self.transport.write_frame(Command::SetSettings, &payload).await?;

        let ack = self.wait_for_ack(Command::SetSettings).await?;
        if let AckStatus::Failure(_) = ack.status {
            return Err(ClockError::CommandRejected {
                command: 0x01,
                status: ack.status,
            });
        }

        Ok(())
    }

    /// Set immediate brightness (preview, 0–10).
    ///
    /// Sends: `02 03 [Value]` to Data Write.
    /// This is a temporary preview and does not persist to settings.
    pub async fn set_brightness(&self, value: Brightness) -> Result<()> {
        let _guard = self.command_mutex.lock().await;

        let payload = [value.nibble()];
        self.transport.write_frame(Command::SetBrightness, &payload).await?;

        let ack = self.wait_for_ack(Command::SetBrightness).await?;
        if let AckStatus::Failure(_) = ack.status {
            return Err(ClockError::CommandRejected {
                command: 0x03,
                status: ack.status,
            });
        }

        Ok(())
    }

    /// Preview ringtone at current or specified volume.
    ///
    /// Sends: `01 04` (current volume) or `02 04 [Volume]` (specific volume)
    /// to Data Write.
    pub async fn preview_ringtone(&self, volume: Option<Volume>) -> Result<()> {
        let _guard = self.command_mutex.lock().await;

        let payload = match volume {
            Some(v) => vec![v.value()],
            None => vec![],
        };
        self.transport.write_frame(Command::PreviewRingtone, &payload).await?;

        let ack = self.wait_for_ack(Command::PreviewRingtone).await?;
        if let AckStatus::Failure(_) = ack.status {
            return Err(ClockError::CommandRejected {
                command: 0x04,
                status: ack.status,
            });
        }

        Ok(())
    }

    /// Disconnect from the device.
    pub async fn disconnect(&self) -> Result<()> {
        self.transport.disconnect().await?;
        self.is_authenticated.store(false, Ordering::SeqCst);
        let _ = self.event_sender.send(ClockEvent::Disconnected);
        Ok(())
    }

    /// Read battery level via the standard GATT Battery Service.
    ///
    /// Reads the Battery Level characteristic (`0x2A19`).
    /// Returns a percentage 0–100.
    pub async fn read_battery(&self) -> Result<BatteryLevel> {
        let data = self.transport.read(CharacteristicUuid::BatteryLevel).await?;
        if data.is_empty() {
            return Err(ClockError::Parse("battery response empty".to_string()));
        }
        Ok(BatteryLevel::new(data[0]))
    }

    /// Upload a custom ringtone to the device.
    ///
    /// Performs the full upload sequence:
    /// 1. MTU exchange to support 130-byte GATT writes.
    /// 2. Audio Init with a 4-byte signature and total size.
    /// 3. Audio Data Packets in 128-byte blocks (4 packets per ACK).
    ///
    /// The audio must be 8-bit unsigned PCM, 8 kHz, mono. Maximum size
    /// is ~98,304 bytes (~12 seconds).
    ///
    /// Sends: `08 10 [SizeLo] [SizeMid] [SizeHi] [Sig0] [Sig1] [Sig2] [Sig3]`
    /// to Data Write, then `81 08 [128B payload]` packets in blocks of 4.
    pub async fn upload_ringtone(&self, audio: &[u8], signature: [u8; 4]) -> Result<()> {
        validate_audio(audio)?;

        let _guard = self.command_mutex.lock().await;

        // Step 1: MTU Exchange — audio packets are 130 bytes, default MTU is 23
        let negotiated_mtu = self.transport.request_mtu(247).await?;
        if (negotiated_mtu as usize) < 132 {
            return Err(ClockError::InvalidSettings(format!("MTU too small for audio upload: {} (need >= 132)", negotiated_mtu)));
        }
        debug!(negotiated_mtu, "MTU exchange successful");

        // Step 2: Audio Init
        let total_size = audio.len() as u32;
        let mut init_frame = Vec::with_capacity(9);
        init_frame.push(0x08); // Length
        init_frame.push(0x10); // Command: Audio Init
        init_frame.extend_from_slice(&total_size.to_le_bytes()[0..3]);
        init_frame.extend_from_slice(&signature);

        self.transport.write(CharacteristicUuid::DataWrite, &init_frame).await?;

        let init_ack = self.wait_for_ack_with_timeout(Command::AudioInit, AUDIO_ACK_TIMEOUT_SECS).await?;
        if let AckStatus::Failure(_) = init_ack.status {
            return Err(ClockError::CommandRejected {
                command: 0x10,
                status: init_ack.status,
            });
        }
        debug!(slot = init_ack.payload, total_size, "audio upload initialized");

        // Step 3: Send data packets in blocks of 4
        let total_packets = audio.len().div_ceil(AUDIO_PACKET_PAYLOAD_SIZE);
        let mut packet_index = 0usize;

        for chunk in audio.chunks(AUDIO_PACKET_PAYLOAD_SIZE) {
            // Pad to 128 bytes with 0xFF
            let mut payload = [0xFFu8; AUDIO_PACKET_PAYLOAD_SIZE];
            payload[..chunk.len()].copy_from_slice(chunk);

            let mut frame = Vec::with_capacity(130);
            frame.push(0x81); // Length (129 bytes follow)
            frame.push(0x08); // Command: Audio Data Packet
            frame.extend_from_slice(&payload);

            self.transport.write(CharacteristicUuid::DataWrite, &frame).await?;

            packet_index += 1;

            // Wait for ACK at end of each block of 4 packets
            if packet_index.is_multiple_of(AUDIO_PACKETS_PER_BLOCK) || packet_index == total_packets {
                let ack = self.wait_for_ack_with_timeout(Command::AudioData, AUDIO_ACK_TIMEOUT_SECS).await?;
                if let AckStatus::Failure(_) = ack.status {
                    return Err(ClockError::CommandRejected {
                        command: 0x08,
                        status: ack.status,
                    });
                }
            }

            if packet_index.is_multiple_of(100) {
                debug!(packet_index, total_packets, "audio upload progress");
            }
        }

        info!(total_packets, "ringtone upload complete");
        Ok(())
    }
}

/// Background notification task.
///
/// Listens for BLE notifications from the connected device, dispatches ACKs
/// to pending request-response channels, broadcasts sensor/battery events,
/// and attempts reconnection on disconnect.
#[allow(clippy::too_many_arguments)]
async fn notification_task(
    transport: Arc<dyn BleTransport>,
    event_sender: broadcast::Sender<ClockEvent>,
    pending: PendingMap,
    is_authenticated: Arc<AtomicBool>,
    auth_token: Arc<Mutex<Option<AuthToken>>>,
    address: MacAddress,
    command_mutex: Arc<Mutex<()>>,
    pending_data_response: Arc<Mutex<Option<mpsc::Sender<Vec<u8>>>>>,
    token_store: Arc<Mutex<Option<Arc<dyn TokenStore>>>>,
) {
    let sensor_uuid = CharacteristicUuid::SensorNotify.uuid();
    let battery_uuid = CharacteristicUuid::BatteryLevel.uuid();

    loop {
        match transport.next_notification().await {
            Some((uuid, value)) => {
                if uuid == sensor_uuid {
                    if let Ok(sensor) = SensorNotification::parse(&value) {
                        let _ = event_sender.send(ClockEvent::SensorUpdate {
                            temperature: sensor.temperature,
                            humidity: sensor.humidity,
                        });
                    }
                } else if uuid == battery_uuid {
                    if let Some(&level) = value.first() {
                        let _ = event_sender.send(ClockEvent::BatteryLevel {
                            level: BatteryLevel::new(level),
                        });
                    }
                } else if let Some(ack) = Ack::parse(&value) {
                    let _ = event_sender.send(ClockEvent::Ack {
                        command: ack.command,
                        status: ack.status,
                    });

                    if let Some(sender) = pop_pending_alive(&pending, ack.command).await {
                        let _ = sender.send(Ok(ack));
                    }
                } else {
                    // Non-ACK data notification — forward to pending data response channel.
                    let sender = {
                        let pending = pending_data_response.lock().await;
                        pending.clone()
                    };
                    if let Some(sender) = sender {
                        let _ = sender.send(value).await;
                    } else {
                        debug!(
                            uuid = %uuid,
                            len = value.len(),
                            "unhandled notification, ignoring"
                        );
                    }
                }
            }
            None => {
                warn!("notification stream ended, device disconnected");
                let _ = event_sender.send(ClockEvent::Disconnected);
                is_authenticated.store(false, Ordering::SeqCst);

                let device = ClockDevice {
                    transport: transport.clone(),
                    address,
                    event_sender: event_sender.clone(),
                    command_mutex: command_mutex.clone(),
                    auth_token: auth_token.clone(),
                    is_authenticated: is_authenticated.clone(),
                    pending: pending.clone(),
                    pending_data_response: pending_data_response.clone(),
                    token_store: token_store.clone(),
                };

                match reconnect_and_restore(&device, 6).await {
                    Ok(()) => {
                        info!("reconnect successful, resuming notification task");
                        continue;
                    }
                    Err(e) => {
                        warn!("reconnect failed after all attempts: {e:?}");
                        break;
                    }
                }
            }
        }
    }
}

/// Reconnect with exponential backoff and full state recovery.
///
/// Delay sequence: 1s, 2s, 4s, 8s, 16s, 32s (capped).
///
/// After a successful BLE connect, re-subscribes to all GATT notify
/// characteristics and re-authenticates with the stored token. This
/// ensures the device is fully operational before commands resume.
async fn reconnect_and_restore(device: &ClockDevice, max_attempts: u32) -> Result<()> {
    let mut delay = Duration::from_secs(1);
    let max_delay = Duration::from_secs(32);

    for attempt in 1..=max_attempts {
        debug!(attempt, delay_ms = delay.as_millis(), "attempting reconnect");

        // Step 1: BLE Reconnect
        if device.transport.connect(&device.address).await.is_err() {
            tokio::time::sleep(delay).await;
            delay = (delay * 2).min(max_delay);
            continue;
        }

        // Step 2: Re-subscribe to all notify characteristics
        let characteristics = [CharacteristicUuid::AuthNotify, CharacteristicUuid::DataNotify, CharacteristicUuid::SensorNotify];

        let mut all_subscribed = true;
        for char_uuid in &characteristics {
            if device.transport.subscribe(*char_uuid).await.is_err() {
                all_subscribed = false;
            }
        }

        if !all_subscribed {
            warn!("reconnect: GATT re-subscription failed, retrying");
            tokio::time::sleep(delay).await;
            delay = (delay * 2).min(max_delay);
            continue;
        }

        // Step 3: Re-authenticate with stored token
        let token = device.auth_token.lock().await;
        if let Some(ref token) = *token
            && device.authenticate(token).await.is_err()
        {
            warn!("reconnect: re-authentication failed, retrying");
            tokio::time::sleep(delay).await;
            delay = (delay * 2).min(max_delay);
            continue;
        }

        // Step 4: Mark as authenticated and notify subscribers
        device.is_authenticated.store(true, Ordering::SeqCst);
        let _ = device.event_sender.send(ClockEvent::Reconnected);
        info!("reconnect: state recovery complete");
        return Ok(());
    }

    Err(TransportError::ReconnectFailed.into())
}

#[cfg(test)]
mod tests {
    use super::*;

    use crate::AlarmSlotIndex;
    use crate::ClockTime;
    use crate::DayMask;
    use crate::Language;
    use crate::MockBleTransport;
    use crate::RingtoneSignature;
    use crate::ScreenLightDuration;
    use crate::TemperatureUnit;
    use crate::TimeFormat;
    use crate::Timezone;

    #[test]
    fn parse_ack_valid() {
        let value = [0x04, 0xff, 0x01, 0x00, 0x06];
        let ack = Ack::parse(&value).unwrap();
        assert_eq!(ack.command, CommandId::new(0x01));
        assert_eq!(ack.status, AckStatus::Success);
        assert_eq!(ack.payload, 0x06);
    }

    #[test]
    fn parse_ack_too_short() {
        let value = [0x04, 0xff, 0x01];
        assert!(Ack::parse(&value).is_none());
    }

    #[test]
    fn parse_ack_wrong_prefix() {
        let value = [0x05, 0xff, 0x01, 0x00, 0x06];
        assert!(Ack::parse(&value).is_none());
    }

    #[tokio::test]
    async fn set_alarm_sends_correct_frame() {
        let mock = Arc::new(MockBleTransport::new());
        let addr = MacAddress::parse("AA:BB:CC:DD:EE:FF").unwrap();
        let device = ClockDevice::new(mock.clone(), addr);
        device.spawn_notification_task();

        let alarm = AlarmEntry::new(ClockTime::new(7, 30).unwrap(), DayMask::WEEKDAYS, true, true);
        device.set_alarm(&alarm, AlarmSlotIndex::new(2).unwrap()).await.unwrap();

        let writes = mock.drain_writes().await;
        assert_eq!(writes.len(), 1);
        assert_eq!(writes[0].0, CharacteristicUuid::DataWrite);
        // Frame: [length=0x07] [cmd=0x05] [slot=2] [enabled=1] [hh=7] [mm=30] [days=0x3E] [snooze=1]
        assert_eq!(writes[0].1, vec![0x07, 0x05, 0x02, 0x01, 0x07, 0x1E, 0x3E, 0x01]);
    }

    #[tokio::test]
    async fn delete_alarm_sends_correct_frame() {
        let mock = Arc::new(MockBleTransport::new());
        let addr = MacAddress::parse("AA:BB:CC:DD:EE:FF").unwrap();
        let device = ClockDevice::new(mock.clone(), addr);
        device.spawn_notification_task();

        device.delete_alarm(AlarmSlotIndex::new(5).unwrap()).await.unwrap();

        let writes = mock.drain_writes().await;
        assert_eq!(writes.len(), 1);
        assert_eq!(writes[0].0, CharacteristicUuid::DataWrite);
        // Frame: [length=0x07] [cmd=0x05] [slot=5] FF FF FF FF FF
        assert_eq!(writes[0].1, vec![0x07, 0x05, 0x05, 0xFF, 0xFF, 0xFF, 0xFF, 0xFF]);
    }

    #[tokio::test]
    async fn read_alarms_parses_multi_packet() {
        let mock = Arc::new(MockBleTransport::new());
        let addr = MacAddress::parse("AA:BB:CC:DD:EE:FF").unwrap();
        let device = ClockDevice::new(mock.clone(), addr);
        device.spawn_notification_task();

        // Pre-push 6 data notify packets on Data Notify.
        // The auto-ACK for the ReadAlarms write will arrive first, then these data packets.
        // Each packet: [length] [0x06] [base_index] [entry1 5B] [entry2 5B] [entry3 5B]
        let data_notify = CharacteristicUuid::DataNotify.uuid();

        // Packet 0: slots 0-2, slot 0 has alarm at 07:30 weekdays
        mock.push_notification(
            data_notify,
            vec![
                0x11, 0x06, 0x00, 0x01, 0x07, 0x1E, 0x3E, 0x01, // slot 0: enabled, 7:30, weekdays, snooze
                0xFF, 0xFF, 0xFF, 0xFF, 0xFF, // slot 1: empty
                0xFF, 0xFF, 0xFF, 0xFF, 0xFF, // slot 2: empty
            ],
        );
        // Packets 1-5: all empty slots
        for base in [3u8, 6, 9, 12, 15] {
            mock.push_notification(
                data_notify,
                vec![
                    0x11, 0x06, base, 0xFF, 0xFF, 0xFF, 0xFF, 0xFF, 0xFF, 0xFF, 0xFF, 0xFF, 0xFF, 0xFF, 0xFF, 0xFF, 0xFF, 0xFF,
                ],
            );
        }

        let slots = device.read_alarms().await.unwrap();
        assert_eq!(slots.len(), 1);
        assert_eq!(slots[0].index.value(), 0);
        assert_eq!(slots[0].entry.hour(), 7);
        assert_eq!(slots[0].entry.minute(), 30);
        assert_eq!(slots[0].entry.repeat_mask(), DayMask::WEEKDAYS);
        assert!(slots[0].entry.enabled());
        assert!(slots[0].entry.snooze());
    }

    #[tokio::test]
    async fn set_alarm_rejects_invalid_slot() {
        let result = AlarmSlotIndex::new(16);
        assert!(result.is_err());
    }

    #[tokio::test]
    async fn write_settings_sends_correct_frame() {
        let mock = Arc::new(MockBleTransport::new());
        let addr = MacAddress::parse("AA:BB:CC:DD:EE:FF").unwrap();
        let device = ClockDevice::new(mock.clone(), addr);
        device.spawn_notification_task();

        let settings = DeviceSettings::new(
            Volume::new(3).unwrap(),
            TimeFormat::TwentyFourHour,
            TemperatureUnit::Celsius,
            Language::English,
            Timezone::from_hours(1).unwrap(),
            ScreenLightDuration::new(10).unwrap(),
            Brightness::new(80).unwrap(),
            Brightness::new(30).unwrap(),
            ClockTime::new(22, 0).unwrap(),
            ClockTime::new(7, 0).unwrap(),
            true,
            true,
            RingtoneSignature::Unused,
        )
        .unwrap();

        device.write_settings(&settings).await.unwrap();

        let writes = mock.drain_writes().await;
        assert_eq!(writes.len(), 1);
        assert_eq!(writes[0].0, CharacteristicUuid::DataWrite);
        // Frame: [length=0x13] [cmd=0x01] [18 bytes payload]
        let frame = &writes[0].1;
        assert_eq!(frame[0], 0x13);
        assert_eq!(frame[1], 0x01);
        assert_eq!(frame.len(), 20);
        // Check some payload values
        assert_eq!(frame[2], 3); // volume
        assert_eq!(frame[3], 0x58); // hdr1
        assert_eq!(frame[4], 0x02); // hdr2
        assert_eq!(frame[5], 0x11); // flags: English | master_alarm_disable
        assert_eq!(frame[8], 0x83); // packed brightness: day=8, night=3
    }

    #[tokio::test]
    async fn read_settings_parses_response() {
        let mock = Arc::new(MockBleTransport::new());
        let addr = MacAddress::parse("AA:BB:CC:DD:EE:FF").unwrap();
        let device = ClockDevice::new(mock.clone(), addr);
        device.spawn_notification_task();

        // Push a settings response on Data Notify.
        // Format: [length=0x13] [cmd=0x02] [18 bytes payload]
        let data_notify = CharacteristicUuid::DataNotify.uuid();
        let mut response = vec![0x13, 0x02];
        // Payload: 18 bytes
        response.extend_from_slice(&[
            3,    // volume
            0x58, // hdr1
            0x02, // hdr2
            0x11, // flags: English | master_alarm_disable
            10,   // timezone: 60 min / 6 = 10
            10,   // screen duration
            0x83, // brightness: day=8, night=3
            22,   // night start hour
            0,    // night start minute
            7,    // night end hour
            0,    // night end minute
            0x01, // timezone sign: positive
            0x01, // night mode enabled
            0xFF, // reserved
            0xFF, 0xFF, 0xFF, 0xFF, // ringtone signature (unused)
        ]);
        mock.push_notification(data_notify, response);

        let settings = device.read_settings().await.unwrap();
        assert_eq!(settings.volume(), crate::Volume::new(3).unwrap());
        assert_eq!(settings.language(), crate::Language::English);
        assert_eq!(settings.time_format(), crate::TimeFormat::TwentyFourHour);
        assert_eq!(settings.temperature_unit(), crate::TemperatureUnit::Celsius);
        assert_eq!(settings.timezone().minutes(), 60);
        assert_eq!(settings.screen_light_duration().seconds(), 10);
        assert_eq!(settings.brightness().value(), 80);
        assert_eq!(settings.night_brightness().value(), 30);
        assert_eq!(settings.night_start().hour(), 22);
        assert_eq!(settings.night_end().hour(), 7);
        assert!(settings.night_mode_enabled());
        assert!(settings.master_alarm_disabled());
        assert!(settings.ringtone_signature().is_unused());
    }

    #[tokio::test]
    async fn set_brightness_sends_correct_frame() {
        let mock = Arc::new(MockBleTransport::new());
        let addr = MacAddress::parse("AA:BB:CC:DD:EE:FF").unwrap();
        let device = ClockDevice::new(mock.clone(), addr);
        device.spawn_notification_task();

        device.set_brightness(crate::Brightness::new(70).unwrap()).await.unwrap();

        let writes = mock.drain_writes().await;
        assert_eq!(writes.len(), 1);
        assert_eq!(writes[0].0, CharacteristicUuid::DataWrite);
        // Frame: [length=0x02] [cmd=0x03] [nibble=7]
        assert_eq!(writes[0].1, vec![0x02, 0x03, 0x07]);
    }

    #[tokio::test]
    async fn preview_ringtone_current_volume() {
        let mock = Arc::new(MockBleTransport::new());
        let addr = MacAddress::parse("AA:BB:CC:DD:EE:FF").unwrap();
        let device = ClockDevice::new(mock.clone(), addr);
        device.spawn_notification_task();

        device.preview_ringtone(None).await.unwrap();

        let writes = mock.drain_writes().await;
        assert_eq!(writes.len(), 1);
        assert_eq!(writes[0].0, CharacteristicUuid::DataWrite);
        // Frame: [length=0x01] [cmd=0x04]
        assert_eq!(writes[0].1, vec![0x01, 0x04]);
    }

    #[tokio::test]
    async fn preview_ringtone_specific_volume() {
        let mock = Arc::new(MockBleTransport::new());
        let addr = MacAddress::parse("AA:BB:CC:DD:EE:FF").unwrap();
        let device = ClockDevice::new(mock.clone(), addr);
        device.spawn_notification_task();

        device.preview_ringtone(Some(crate::Volume::new(3).unwrap())).await.unwrap();

        let writes = mock.drain_writes().await;
        assert_eq!(writes.len(), 1);
        assert_eq!(writes[0].0, CharacteristicUuid::DataWrite);
        // Frame: [length=0x02] [cmd=0x04] [volume=3]
        assert_eq!(writes[0].1, vec![0x02, 0x04, 0x03]);
    }

    #[tokio::test]
    async fn read_battery_returns_level() {
        let mock = Arc::new(MockBleTransport::new());
        let addr = MacAddress::parse("AA:BB:CC:DD:EE:FF").unwrap();
        let device = ClockDevice::new(mock.clone(), addr);
        device.spawn_notification_task();

        mock.set_read_value(CharacteristicUuid::BatteryLevel, vec![85]).await;

        let level = device.read_battery().await.unwrap();
        assert_eq!(level.value(), 85);
    }

    #[tokio::test]
    async fn read_battery_empty_response_errors() {
        let mock = Arc::new(MockBleTransport::new());
        let addr = MacAddress::parse("AA:BB:CC:DD:EE:FF").unwrap();
        let device = ClockDevice::new(mock.clone(), addr);
        device.spawn_notification_task();

        mock.set_read_value(CharacteristicUuid::BatteryLevel, vec![]).await;

        assert!(device.read_battery().await.is_err());
    }

    #[test]
    fn validate_audio_empty_rejected() {
        assert!(validate_audio(&[]).is_err());
    }

    #[test]
    fn validate_audio_valid() {
        assert!(validate_audio(&[0x80; 256]).is_ok());
    }

    #[test]
    fn validate_audio_too_large_rejected() {
        let audio = vec![0u8; AUDIO_MAX_SIZE + 1];
        assert!(validate_audio(&audio).is_err());
    }

    #[test]
    fn validate_audio_max_size_accepted() {
        let audio = vec![0u8; AUDIO_MAX_SIZE];
        assert!(validate_audio(&audio).is_ok());
    }

    #[tokio::test]
    async fn upload_ringtone_small_audio() {
        let mock = Arc::new(MockBleTransport::new());
        let addr = MacAddress::parse("AA:BB:CC:DD:EE:FF").unwrap();
        let device = ClockDevice::new(mock.clone(), addr);
        device.spawn_notification_task();

        // 128 bytes = 1 packet (1 block, ACK after last packet)
        let audio = vec![0xAAu8; 128];
        let signature = [0x01, 0x02, 0x03, 0x04];

        device.upload_ringtone(&audio, signature).await.unwrap();

        let writes = mock.drain_writes().await;
        // Write 0: Audio Init frame
        assert_eq!(writes[0].0, CharacteristicUuid::DataWrite);
        assert_eq!(writes[0].1[0], 0x08); // length
        assert_eq!(writes[0].1[1], 0x10); // command
        assert_eq!(writes[0].1[2], 128); // size lo
        assert_eq!(writes[0].1[3], 0); // size mid
        assert_eq!(writes[0].1[4], 0); // size hi
        assert_eq!(&writes[0].1[5..9], &signature);

        // Write 1: Audio Data Packet
        assert_eq!(writes[1].0, CharacteristicUuid::DataWrite);
        assert_eq!(writes[1].1[0], 0x81); // length
        assert_eq!(writes[1].1[1], 0x08); // command
        assert_eq!(&writes[1].1[2..], &[0xAA; 128]); // payload
    }

    #[tokio::test]
    async fn upload_ringtone_multi_block() {
        let mock = Arc::new(MockBleTransport::new());
        let addr = MacAddress::parse("AA:BB:CC:DD:EE:FF").unwrap();
        let device = ClockDevice::new(mock.clone(), addr);
        device.spawn_notification_task();

        // 512 bytes = 4 packets = 1 full block (ACK after 4th)
        let audio = vec![0x55u8; 512];
        let signature = [0xBA, 0x2C, 0x2C, 0x8C];

        device.upload_ringtone(&audio, signature).await.unwrap();

        let writes = mock.drain_writes().await;
        // 1 init + 4 data packets = 5 writes
        assert_eq!(writes.len(), 5);
        // Verify all data packets have correct framing
        for i in 1..=4 {
            assert_eq!(writes[i].1[0], 0x81);
            assert_eq!(writes[i].1[1], 0x08);
            assert_eq!(writes[i].1.len(), 130);
        }
    }

    #[tokio::test]
    async fn upload_ringtone_pads_last_packet() {
        let mock = Arc::new(MockBleTransport::new());
        let addr = MacAddress::parse("AA:BB:CC:DD:EE:FF").unwrap();
        let device = ClockDevice::new(mock.clone(), addr);
        device.spawn_notification_task();

        // 130 bytes = 2 packets: 128 + 2 (padded to 128 with 0xFF)
        let audio = vec![0x11u8; 130];
        let signature = [0x00; 4];

        device.upload_ringtone(&audio, signature).await.unwrap();

        let writes = mock.drain_writes().await;
        // 1 init + 2 data packets = 3 writes
        assert_eq!(writes.len(), 3);
        // Second data packet: first 2 bytes are 0x11, rest 0xFF
        assert_eq!(writes[2].1[0], 0x81);
        assert_eq!(writes[2].1[1], 0x08);
        assert_eq!(writes[2].1[2], 0x11);
        assert_eq!(writes[2].1[3], 0x11);
        assert_eq!(writes[2].1[4], 0xFF);
        assert_eq!(writes[2].1[129], 0xFF);
    }

    #[tokio::test]
    async fn upload_ringtone_empty_audio_errors() {
        let mock = Arc::new(MockBleTransport::new());
        let addr = MacAddress::parse("AA:BB:CC:DD:EE:FF").unwrap();
        let device = ClockDevice::new(mock.clone(), addr);
        device.spawn_notification_task();

        let result = device.upload_ringtone(&[], [0x01, 0x02, 0x03, 0x04]).await;
        assert!(result.is_err());
    }
}
