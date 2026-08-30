use cgd1_rs::MacAddress;

use serde::Serialize;

/// Response: device connected.
#[derive(Debug, Clone, Serialize)]
pub struct ConnectResponse {
    /// Device MAC address.
    pub address: MacAddress,
    /// Whether the device is now connected.
    pub connected: bool,
}
