use std::collections::HashMap;
use std::collections::VecDeque;
use std::sync::Arc;
use std::sync::atomic::AtomicBool;
use std::sync::atomic::Ordering;
use std::time::Duration;

use tokio::sync::Mutex;
use tokio::sync::broadcast;
use tokio::sync::oneshot;
use tokio::time::timeout;
use tracing::debug;
use tracing::info;
use tracing::warn;

use crate::BleTransport;
use crate::CharacteristicUuid;
use crate::command::Ack;
use crate::command::AckStatus;
use crate::command::Command;
use crate::command::CommandId;
use crate::error::ClockError;
use crate::error::Result;
use crate::event::ClockEvent;
use crate::token::AuthToken;
use crate::types::BatteryLevel;
use crate::types::Humidity;
use crate::types::MacAddress;
use crate::types::Temperature;

/// Response timeout in seconds.
const RESPONSE_TIMEOUT_SECS: u64 = 10;

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

        tokio::spawn(async move { notification_task(transport, event_sender, pending, is_authenticated, auth_token, address, command_mutex).await });
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
            Ok(Err(_)) => Err(ClockError::Transport("pending request canceled".into())),
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
            return Err(ClockError::AuthFailed(format!("init status: {code:#04x}")));
        }

        // Step 2: Auth Confirm
        self.transport.write_frame(Command::AuthConfirm, token.payload()).await?;

        let ack = self.wait_for_ack(Command::AuthConfirm).await?;
        if let AckStatus::Failure(code) = ack.status {
            return Err(ClockError::AuthFailed(format!("confirm status: {code:#04x}")));
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

        Ok(())
    }

    /// Disconnect from the device.
    pub async fn disconnect(&self) -> Result<()> {
        self.transport.disconnect().await?;
        self.is_authenticated.store(false, Ordering::SeqCst);
        let _ = self.event_sender.send(ClockEvent::Disconnected);
        Ok(())
    }
}

/// Parse sensor data from a Sensor Notify notification.
///
/// Format: `[00] [Temp L] [Temp H] [Hum L] [Hum H]` (5 bytes).
fn parse_sensor_data(value: &[u8]) -> Option<(Temperature, Humidity)> {
    if value.len() >= 5 {
        let temp = i16::from_le_bytes([value[1], value[2]]) as f32 / 100.0;
        let humidity = u16::from_le_bytes([value[3], value[4]]) as f32 / 100.0;
        Some((Temperature::new(temp), Humidity::new(humidity)))
    } else {
        None
    }
}

/// Background notification task.
///
/// Listens for BLE notifications from the connected device, dispatches ACKs
/// to pending request-response channels, broadcasts sensor/battery events,
/// and attempts reconnection on disconnect.
async fn notification_task(
    transport: Arc<dyn BleTransport>,
    event_sender: broadcast::Sender<ClockEvent>,
    pending: PendingMap,
    is_authenticated: Arc<AtomicBool>,
    auth_token: Arc<Mutex<Option<AuthToken>>>,
    address: MacAddress,
    command_mutex: Arc<Mutex<()>>,
) {
    let sensor_uuid = CharacteristicUuid::SensorNotify.uuid();
    let battery_uuid = CharacteristicUuid::BatteryLevel.uuid();

    loop {
        match transport.next_notification().await {
            Some((uuid, value)) => {
                if uuid == sensor_uuid {
                    if let Some((temp, humidity)) = parse_sensor_data(&value) {
                        let _ = event_sender.send(ClockEvent::SensorUpdate { temperature: temp, humidity });
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
                    debug!(
                        uuid = %uuid,
                        len = value.len(),
                        "unhandled notification, ignoring"
                    );
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

    Err(ClockError::Transport("reconnect with state recovery failed".into()))
}

#[cfg(test)]
mod tests {
    use super::*;

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

    #[test]
    fn parse_sensor_data_valid() {
        let value = [0x00, 0x64, 0x00, 0xC8, 0x00];
        let (temp, humidity) = parse_sensor_data(&value).unwrap();
        assert_eq!(temp.value(), 1.0);
        assert_eq!(humidity.value(), 2.0);
    }

    #[test]
    fn parse_sensor_data_negative_temp() {
        let value = [0x00, 0x9C, 0xFF, 0xC8, 0x00];
        let (temp, humidity) = parse_sensor_data(&value).unwrap();
        assert_eq!(temp.value(), -1.0);
        assert_eq!(humidity.value(), 2.0);
    }

    #[test]
    fn parse_sensor_data_too_short() {
        let value = [0x00, 0x64, 0x00];
        assert!(parse_sensor_data(&value).is_none());
    }
}
