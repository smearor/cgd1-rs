use crate::AdvertisementData;
use crate::MacAddress;

/// A discovered CGD1 device.
#[derive(Debug, Clone)]
pub struct DiscoveredDevice {
    /// MAC address.
    pub address: MacAddress,
    /// Last seen advertisement data (if any).
    pub advertisement: Option<AdvertisementData>,
    /// RSSI signal strength.
    pub rssi: Option<i16>,
}
