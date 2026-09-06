use crate::ble::characteristic::CharacteristicUuid;

/// A BLE notification received from a connected device.
///
/// Contains the characteristic that produced the notification and the
/// raw value bytes. The notification task uses the characteristic to
/// route the value to the appropriate handler (sensor, ACK, data
/// response, etc.).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct BleNotification {
    /// The characteristic that produced this notification.
    pub characteristic: CharacteristicUuid,
    /// The raw notification value bytes.
    pub value: Vec<u8>,
}

impl BleNotification {
    /// Create a new notification from a characteristic and value bytes.
    pub fn new(characteristic: CharacteristicUuid, value: Vec<u8>) -> Self {
        Self { characteristic, value }
    }
}
