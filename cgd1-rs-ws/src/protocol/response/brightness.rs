use cgd1_rs::Brightness;
use cgd1_rs::MacAddress;

use serde::Serialize;

/// Response: brightness set.
#[derive(Debug, Clone, Serialize)]
pub struct BrightnessResponse {
    /// Device MAC address.
    pub address: MacAddress,
    /// Brightness value that was set.
    pub brightness: Brightness,
}
