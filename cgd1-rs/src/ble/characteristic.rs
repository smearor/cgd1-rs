use uuid::Uuid;

/// BLE base UUID bytes for 16-bit UUID expansion.
const BLE_BASE_UUID: [u8; 8] = [0x80, 0x00, 0x00, 0x80, 0x5f, 0x9b, 0x34, 0xfb];

/// CGD1 GATT characteristic identifiers.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum CharacteristicUuid {
    /// Auth Write - `00000001-0000-1000-8000-00805f9b34fb`
    AuthWrite,
    /// Auth Notify - `00000002-0000-1000-8000-00805f9b34fb`
    AuthNotify,
    /// Data Write - `0000000b-0000-1000-8000-00805f9b34fb`
    DataWrite,
    /// Data Notify - `0000000c-0000-1000-8000-00805f9b34fb`
    DataNotify,
    /// Sensor Notify - `00000100-0000-1000-8000-00805f9b34fb`
    SensorNotify,
    /// Battery Level - `0x2a19` (standard GATT)
    BatteryLevel,
}

impl CharacteristicUuid {
    /// Convert to the full 128-bit UUID.
    pub fn uuid(self) -> Uuid {
        match self {
            Self::AuthWrite => Uuid::from_fields(0x00000001, 0x0000, 0x1000, &BLE_BASE_UUID),
            Self::AuthNotify => Uuid::from_fields(0x00000002, 0x0000, 0x1000, &BLE_BASE_UUID),
            Self::DataWrite => Uuid::from_fields(0x0000000b, 0x0000, 0x1000, &BLE_BASE_UUID),
            Self::DataNotify => Uuid::from_fields(0x0000000c, 0x0000, 0x1000, &BLE_BASE_UUID),
            Self::SensorNotify => Uuid::from_fields(0x00000100, 0x0000, 0x1000, &BLE_BASE_UUID),
            Self::BatteryLevel => Uuid::from_fields(0x00002a19, 0x0000, 0x1000, &BLE_BASE_UUID),
        }
    }
}
