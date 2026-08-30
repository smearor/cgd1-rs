use cgd1_rs::MacAddress;

use serde::Serialize;

/// Response: ringtone previewing.
#[derive(Debug, Clone, Serialize)]
pub struct PreviewRingtoneResponse {
    /// Device MAC address.
    pub address: MacAddress,
    /// Whether the ringtone is previewing.
    pub previewing: bool,
}
