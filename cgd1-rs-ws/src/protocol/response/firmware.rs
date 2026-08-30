use cgd1_rs::MacAddress;

use serde::Serialize;

/// Response: firmware version.
#[derive(Debug, Clone, Serialize)]
pub struct FirmwareResponse {
    /// Device MAC address.
    pub address: MacAddress,
    /// Firmware version string.
    pub firmware: String,
}
