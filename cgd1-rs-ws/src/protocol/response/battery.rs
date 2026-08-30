use cgd1_rs::BatteryLevel;
use cgd1_rs::MacAddress;

use serde::Serialize;

/// Response: battery level.
#[derive(Debug, Clone, Serialize)]
pub struct BatteryResponse {
    /// Device MAC address.
    pub address: MacAddress,
    /// Battery level percentage (0–100).
    pub battery: BatteryLevel,
}
