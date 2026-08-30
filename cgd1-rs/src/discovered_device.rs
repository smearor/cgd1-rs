use crate::AdvertisementData;
use crate::MacAddress;

use serde::Deserialize;
use serde::Serialize;

/// A discovered CGD1 device.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DiscoveredDevice {
    /// MAC address.
    pub address: MacAddress,
    /// Last seen advertisement data (if any).
    pub advertisement: Option<AdvertisementData>,
    /// RSSI signal strength.
    pub rssi: Option<i16>,
}
