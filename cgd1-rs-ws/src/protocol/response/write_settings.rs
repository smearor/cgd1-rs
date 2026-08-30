use cgd1_rs::MacAddress;

use serde::Serialize;

/// Response: settings written.
#[derive(Debug, Clone, Serialize)]
pub struct WriteSettingsResponse {
    /// Device MAC address.
    pub address: MacAddress,
    /// Whether the settings were written.
    pub written: bool,
}
