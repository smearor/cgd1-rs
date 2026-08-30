use cgd1_rs::AlarmSlotIndex;
use cgd1_rs::MacAddress;

use serde::Serialize;

/// Response: alarm set.
#[derive(Debug, Clone, Serialize)]
pub struct AlarmSetResponse {
    /// Device MAC address.
    pub address: MacAddress,
    /// Slot index where the alarm was set.
    pub slot: AlarmSlotIndex,
    /// Whether the alarm was set.
    pub set: bool,
}
