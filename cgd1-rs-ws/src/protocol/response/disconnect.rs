use cgd1_rs::MacAddress;

use serde::Serialize;

/// Response: device disconnected.
#[derive(Debug, Clone, Serialize)]
pub struct DisconnectResponse {
    /// Device MAC address.
    pub address: MacAddress,
    /// Whether the device is now disconnected.
    pub disconnected: bool,
}
