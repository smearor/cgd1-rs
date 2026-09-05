use std::fmt::Display;
use std::fmt::Formatter;

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
    /// Device Name - `0x2a00` (standard GATT)
    DeviceName,
    /// Appearance - `0x2a01` (standard GATT)
    Appearance,
    /// Peripheral Preferred Connection Parameters - `0x2a04` (standard GATT)
    PeripheralPreferredConnectionParameters,
    /// Service Changed - `0x2a05` (standard GATT)
    ServiceChanged,
    /// PnP ID - `0x2a50` (standard GATT)
    PnpId,
    /// Firmware Version - `00000004-0000-1000-8000-00805f9b34fb`
    FirmwareVersion,
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
            Self::DeviceName => Uuid::from_fields(0x00002a00, 0x0000, 0x1000, &BLE_BASE_UUID),
            Self::Appearance => Uuid::from_fields(0x00002a01, 0x0000, 0x1000, &BLE_BASE_UUID),
            Self::PeripheralPreferredConnectionParameters => Uuid::from_fields(0x00002a04, 0x0000, 0x1000, &BLE_BASE_UUID),
            Self::ServiceChanged => Uuid::from_fields(0x00002a05, 0x0000, 0x1000, &BLE_BASE_UUID),
            Self::PnpId => Uuid::from_fields(0x00002a50, 0x0000, 0x1000, &BLE_BASE_UUID),
            Self::FirmwareVersion => Uuid::from_fields(0x00000004, 0x0000, 0x1000, &BLE_BASE_UUID),
        }
    }
}

impl Display for CharacteristicUuid {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::AuthWrite => write!(f, "AuthWrite"),
            Self::AuthNotify => write!(f, "AuthNotify"),
            Self::DataWrite => write!(f, "DataWrite"),
            Self::DataNotify => write!(f, "DataNotify"),
            Self::SensorNotify => write!(f, "SensorNotify"),
            Self::BatteryLevel => write!(f, "BatteryLevel"),
            Self::DeviceName => write!(f, "DeviceName"),
            Self::Appearance => write!(f, "Appearance"),
            Self::PeripheralPreferredConnectionParameters => write!(f, "PeripheralPreferredConnectionParameters"),
            Self::ServiceChanged => write!(f, "ServiceChanged"),
            Self::PnpId => write!(f, "PnpId"),
            Self::FirmwareVersion => write!(f, "FirmwareVersion"),
        }
    }
}

impl TryFrom<Uuid> for CharacteristicUuid {
    type Error = Uuid;

    /// Convert a raw 128-bit UUID into the corresponding [`CharacteristicUuid`].
    ///
    /// Returns the original UUID as an error if it does not match any known
    /// CGD1 characteristic.
    fn try_from(uuid: Uuid) -> Result<Self, Self::Error> {
        let auth_write = Self::AuthWrite.uuid();
        let auth_notify = Self::AuthNotify.uuid();
        let data_write = Self::DataWrite.uuid();
        let data_notify = Self::DataNotify.uuid();
        let sensor_notify = Self::SensorNotify.uuid();
        let battery_level = Self::BatteryLevel.uuid();
        let device_name = Self::DeviceName.uuid();
        let appearance = Self::Appearance.uuid();
        let ppcp = Self::PeripheralPreferredConnectionParameters.uuid();
        let service_changed = Self::ServiceChanged.uuid();
        let pnp_id = Self::PnpId.uuid();
        let firmware_version = Self::FirmwareVersion.uuid();

        match uuid {
            u if u == auth_write => Ok(Self::AuthWrite),
            u if u == auth_notify => Ok(Self::AuthNotify),
            u if u == data_write => Ok(Self::DataWrite),
            u if u == data_notify => Ok(Self::DataNotify),
            u if u == sensor_notify => Ok(Self::SensorNotify),
            u if u == battery_level => Ok(Self::BatteryLevel),
            u if u == device_name => Ok(Self::DeviceName),
            u if u == appearance => Ok(Self::Appearance),
            u if u == ppcp => Ok(Self::PeripheralPreferredConnectionParameters),
            u if u == service_changed => Ok(Self::ServiceChanged),
            u if u == pnp_id => Ok(Self::PnpId),
            u if u == firmware_version => Ok(Self::FirmwareVersion),
            _ => Err(uuid),
        }
    }
}
