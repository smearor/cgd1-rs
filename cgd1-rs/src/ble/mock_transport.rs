use async_trait::async_trait;
use std::collections::VecDeque;
use std::sync::atomic::AtomicBool;
use std::sync::atomic::Ordering;
use tokio::sync::Mutex;
use uuid::Uuid;

use crate::ble::advertisement::AdvertisementData;
use crate::ble::characteristic::CharacteristicUuid;
use crate::ble::transport::BleTransport;
use crate::error::ClockError;
use crate::error::Result;
use crate::types::MacAddress;

/// In-memory mock BLE transport for testing.
///
/// Captures written data so tests can assert on frame content, and allows
/// injecting notification values to simulate device responses.
pub struct MockBleTransport {
    connected: AtomicBool,
    writes: Mutex<VecDeque<(CharacteristicUuid, Vec<u8>)>>,
    notifications: Mutex<VecDeque<(Uuid, Vec<u8>)>>,
    advertisements: Mutex<VecDeque<AdvertisementData>>,
    subscribed: Mutex<Vec<CharacteristicUuid>>,
    read_values: Mutex<Vec<(CharacteristicUuid, Vec<u8>)>>,
    mtu: Mutex<u16>,
}

impl MockBleTransport {
    /// Create a new mock transport.
    pub fn new() -> Self {
        Self {
            connected: AtomicBool::new(false),
            writes: Mutex::new(VecDeque::new()),
            notifications: Mutex::new(VecDeque::new()),
            advertisements: Mutex::new(VecDeque::new()),
            subscribed: Mutex::new(Vec::new()),
            read_values: Mutex::new(Vec::new()),
            mtu: Mutex::new(23),
        }
    }

    /// Push a notification that will be returned by `next_notification`.
    pub async fn push_notification(&self, uuid: Uuid, data: Vec<u8>) {
        self.notifications.lock().await.push_back((uuid, data));
    }

    /// Push an advertisement that will be returned by `next_advertisement`.
    pub async fn push_advertisement(&self, adv: AdvertisementData) {
        self.advertisements.lock().await.push_back(adv);
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
        self.advertisements.lock().await.pop_front()
    }

    async fn connect(&self, _address: &MacAddress) -> Result<()> {
        self.connected.store(true, Ordering::SeqCst);
        Ok(())
    }

    async fn disconnect(&self) -> Result<()> {
        self.connected.store(false, Ordering::SeqCst);
        Ok(())
    }

    async fn write(&self, characteristic: CharacteristicUuid, data: &[u8]) -> Result<()> {
        self.writes.lock().await.push_back((characteristic, data.to_vec()));
        Ok(())
    }

    async fn subscribe(&self, characteristic: CharacteristicUuid) -> Result<()> {
        self.subscribed.lock().await.push(characteristic);
        Ok(())
    }

    async fn next_notification(&self) -> Option<(Uuid, Vec<u8>)> {
        self.notifications.lock().await.pop_front()
    }

    async fn read(&self, characteristic: CharacteristicUuid) -> Result<Vec<u8>> {
        let mut read_values = self.read_values.lock().await;
        let pos = read_values.iter().position(|(c, _)| *c == characteristic);
        if let Some(idx) = pos {
            Ok(read_values.remove(idx).1)
        } else {
            Err(ClockError::Transport("no read value set for characteristic".into()))
        }
    }

    async fn request_mtu(&self, mtu: u16) -> Result<u16> {
        let mut current = self.mtu.lock().await;
        *current = mtu;
        Ok(mtu)
    }

    fn is_connected(&self) -> bool {
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
        transport.write(CharacteristicUuid::AuthWrite, &[0x11, 0x01]).await.unwrap();
        let writes = transport.drain_writes().await;
        assert_eq!(writes.len(), 1);
        assert_eq!(writes[0].0, CharacteristicUuid::AuthWrite);
        assert_eq!(writes[0].1, vec![0x11, 0x01]);
    }

    #[tokio::test]
    async fn mock_notification_push_pop() {
        let transport = MockBleTransport::new();
        let uuid = CharacteristicUuid::AuthNotify.uuid();
        transport.push_notification(uuid, vec![0x04, 0xff, 0x01, 0x00, 0x00]).await;
        let (recv_uuid, data) = transport.next_notification().await.unwrap();
        assert_eq!(recv_uuid, uuid);
        assert_eq!(data, vec![0x04, 0xff, 0x01, 0x00, 0x00]);
    }

    #[tokio::test]
    async fn mock_connect_disconnect() {
        let transport = MockBleTransport::new();
        assert!(!transport.is_connected());
        let addr = MacAddress::parse("AA:BB:CC:DD:EE:FF").unwrap();
        transport.connect(&addr).await.unwrap();
        assert!(transport.is_connected());
        transport.disconnect().await.unwrap();
        assert!(!transport.is_connected());
    }

    #[tokio::test]
    async fn mock_subscribe_tracking() {
        let transport = MockBleTransport::new();
        transport.subscribe(CharacteristicUuid::AuthNotify).await.unwrap();
        transport.subscribe(CharacteristicUuid::DataNotify).await.unwrap();
        let subs = transport.subscribed().await;
        assert_eq!(subs, vec![CharacteristicUuid::AuthNotify, CharacteristicUuid::DataNotify]);
    }
}
