use cgd1_rs::MacAddress;

use serde::Serialize;

/// Response: time synchronized.
#[derive(Debug, Clone, Serialize)]
pub struct SyncTimeResponse {
    /// Device MAC address.
    pub address: MacAddress,
    /// Whether the time was synchronized.
    pub synced: bool,
}
