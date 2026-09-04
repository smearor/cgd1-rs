use std::sync::atomic::AtomicBool;
use std::sync::atomic::Ordering;

use async_trait::async_trait;
use tokio::sync::Mutex;
use tokio::sync::mpsc;
use uuid::Uuid;

use crate::ble::advertisement::AdvertisementData;
use crate::ble::characteristic::CharacteristicUuid;
use crate::ble::notification::BleNotification;
use crate::ble::transport::BleTransport;
use crate::error::Result;
use crate::error::TransportError;
use crate::types::MacAddress;

/// In-memory mock BLE transport for testing.
///
/// Captures written data so tests can assert on frame content, and allows
/// injecting notification values to simulate device responses.
/// When a command frame is written, an ACK is automatically generated on the
/// corresponding notify characteristic.
pub struct MockBleTransport {
    connected: AtomicBool,
    writes: Mutex<Vec<(CharacteristicUuid, Vec<u8>)>>,
    notifications_tx: mpsc::UnboundedSender<BleNotification>,
    notifications_rx: Mutex<mpsc::UnboundedReceiver<BleNotification>>,
    advertisements: Mutex<Vec<AdvertisementData>>,
    subscribed: Mutex<Vec<CharacteristicUuid>>,
    read_values: Mutex<Vec<(CharacteristicUuid, Vec<u8>)>>,
    mtu: Mutex<u16>,
    auto_ack: AtomicBool,
}

impl MockBleTransport {
    /// Create a new mock transport with auto-ACK enabled.
    pub fn new() -> Self {
        let (tx, rx) = mpsc::unbounded_channel();
        Self {
            connected: AtomicBool::new(false),
            writes: Mutex::new(Vec::new()),
            notifications_tx: tx,
            notifications_rx: Mutex::new(rx),
            advertisements: Mutex::new(Vec::new()),
            subscribed: Mutex::new(Vec::new()),
            read_values: Mutex::new(Vec::new()),
            mtu: Mutex::new(23),
            auto_ack: AtomicBool::new(true),
        }
    }

    /// Push a notification that will be returned by `next_notification`.
    pub fn push_notification(&self, notification: BleNotification) {
        let _ = self.notifications_tx.send(notification);
    }

    /// Push an advertisement that will be returned by `next_advertisement`.
    pub async fn push_advertisement(&self, adv: AdvertisementData) {
        self.advertisements.lock().await.push(adv);
    }

    /// Set the value returned by `read` for a given characteristic.
    pub async fn set_read_value(&self, characteristic: CharacteristicUuid, data: Vec<u8>) {
        self.read_values.lock().await.push((characteristic, data));
    }

    /// Drain and return all captured writes.
    pub async fn drain_writes(&self) -> Vec<(CharacteristicUuid, Vec<u8>)> {
        self.writes.lock().await.drain(..).collect()
    }

    /// Get the list of subscribed characteristics.
    pub async fn subscribed(&self) -> Vec<CharacteristicUuid> {
        self.subscribed.lock().await.clone()
    }

    /// Enable or disable auto-ACK generation on writes.
    pub fn set_auto_ack(&self, enabled: bool) {
        self.auto_ack.store(enabled, Ordering::SeqCst);
    }

    /// Determine the notify characteristic for a given write characteristic.
    fn notify_for(characteristic: CharacteristicUuid) -> Option<CharacteristicUuid> {
        match characteristic {
            CharacteristicUuid::AuthWrite => Some(CharacteristicUuid::AuthNotify),
            CharacteristicUuid::DataWrite => Some(CharacteristicUuid::DataNotify),
            _ => None,
        }
    }

    /// Auto-generate an ACK for a command frame if auto-ACK is enabled.
    ///
    /// Frame format: `[length] [command_byte] [payload...]`
    /// ACK format: `[0x04] [0xff] [command_byte] [0x00] [0x00]`
    fn maybe_auto_ack(&self, characteristic: CharacteristicUuid, data: &[u8]) {
        if !self.auto_ack.load(Ordering::SeqCst) {
            return;
        }
        if data.len() < 2 {
            return;
        }
        let Some(notify) = Self::notify_for(characteristic) else {
            return;
        };
        let command_byte = data[1];
        let ack = vec![0x04, 0xff, command_byte, 0x00, 0x00];
        self.push_notification(BleNotification::new(notify, ack));
    }
}

impl Default for MockBleTransport {
    fn default() -> Self {
        Self::new()
    }
}

#[async_trait]
impl BleTransport for MockBleTransport {
    async fn start_scan(&self, _filter_uuid: Uuid) -> Result<()> {
        Ok(())
    }

    async fn stop_scan(&self) -> Result<()> {
        Ok(())
    }

    async fn next_advertisement(&self) -> Option<AdvertisementData> {
        self.advertisements.lock().await.pop()
    }

    async fn connect(&self, _address: &MacAddress) -> Result<()> {
        self.connected.store(true, Ordering::SeqCst);
        Ok(())
    }

    async fn disconnect(&self, _address: &MacAddress) -> Result<()> {
        self.connected.store(false, Ordering::SeqCst);
        Ok(())
    }

    async fn write(&self, _address: &MacAddress, characteristic: CharacteristicUuid, data: &[u8]) -> Result<()> {
        self.writes.lock().await.push((characteristic, data.to_vec()));
        self.maybe_auto_ack(characteristic, data);
        Ok(())
    }

    async fn subscribe(&self, _address: &MacAddress, characteristic: CharacteristicUuid) -> Result<()> {
        self.subscribed.lock().await.push(characteristic);
        Ok(())
    }

    async fn next_notification(&self, _address: &MacAddress) -> Option<BleNotification> {
        self.notifications_rx.lock().await.recv().await
    }

    async fn read(&self, _address: &MacAddress, characteristic: CharacteristicUuid) -> Result<Vec<u8>> {
        let mut read_values = self.read_values.lock().await;
        let pos = read_values.iter().position(|(c, _)| *c == characteristic);
        if let Some(idx) = pos {
            Ok(read_values.remove(idx).1)
        } else {
            Err(TransportError::NoReadValue.into())
        }
    }

    async fn request_mtu(&self, _address: &MacAddress, mtu: u16) -> Result<u16> {
        let mut current = self.mtu.lock().await;
        *current = mtu;
        Ok(mtu)
    }

    fn is_connected(&self, _address: &MacAddress) -> bool {
        self.connected.load(Ordering::SeqCst)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::CharacteristicUuid;

    #[tokio::test]
    async fn mock_write_capture() {
        let transport = MockBleTransport::new();
        transport.set_auto_ack(false);
        let addr = MacAddress::parse("AA:BB:CC:DD:EE:FF").unwrap();
        transport.write(&addr, CharacteristicUuid::AuthWrite, &[0x11, 0x01]).await.unwrap();
        let writes = transport.drain_writes().await;
        assert_eq!(writes.len(), 1);
        assert_eq!(writes[0].0, CharacteristicUuid::AuthWrite);
        assert_eq!(writes[0].1, vec![0x11, 0x01]);
    }

    #[tokio::test]
    async fn mock_notification_push_pop() {
        let transport = MockBleTransport::new();
        let characteristic = CharacteristicUuid::AuthNotify;
        transport.push_notification(BleNotification::new(characteristic, vec![0x04, 0xff, 0x01, 0x00, 0x00]));
        let addr = MacAddress::parse("AA:BB:CC:DD:EE:FF").unwrap();
        let notif = transport.next_notification(&addr).await.unwrap();
        assert_eq!(notif.characteristic, characteristic);
        assert_eq!(notif.value, vec![0x04, 0xff, 0x01, 0x00, 0x00]);
    }

    #[tokio::test]
    async fn mock_connect_disconnect() {
        let transport = MockBleTransport::new();
        let addr = MacAddress::parse("AA:BB:CC:DD:EE:FF").unwrap();
        assert!(!transport.is_connected(&addr));
        transport.connect(&addr).await.unwrap();
        assert!(transport.is_connected(&addr));
        transport.disconnect(&addr).await.unwrap();
        assert!(!transport.is_connected(&addr));
    }

    #[tokio::test]
    async fn mock_subscribe_tracking() {
        let transport = MockBleTransport::new();
        let addr = MacAddress::parse("AA:BB:CC:DD:EE:FF").unwrap();
        transport.subscribe(&addr, CharacteristicUuid::AuthNotify).await.unwrap();
        transport.subscribe(&addr, CharacteristicUuid::DataNotify).await.unwrap();
        let subs = transport.subscribed().await;
        assert_eq!(subs, vec![CharacteristicUuid::AuthNotify, CharacteristicUuid::DataNotify]);
    }

    #[tokio::test]
    async fn mock_auto_ack() {
        let transport = MockBleTransport::new();
        let addr = MacAddress::parse("AA:BB:CC:DD:EE:FF").unwrap();
        transport
            .write(&addr, CharacteristicUuid::DataWrite, &[0x07, 0x05, 0x02, 0x01, 0x07, 0x1E, 0x3E, 0x01])
            .await
            .unwrap();
        let notif = transport.next_notification(&addr).await.unwrap();
        assert_eq!(notif.characteristic, CharacteristicUuid::DataNotify);
        assert_eq!(notif.value, vec![0x04, 0xff, 0x05, 0x00, 0x00]);
    }
}
