use cgd1_rs::AlarmSlotIndex;
use cgd1_rs::MacAddress;

use serde::Serialize;

/// Response: alarm deleted.
#[derive(Debug, Clone, Serialize)]
pub struct AlarmDeleteResponse {
    /// Device MAC address.
    pub address: MacAddress,
    /// Slot index from which the alarm was deleted.
    pub slot: AlarmSlotIndex,
    /// Whether the alarm was deleted.
    pub deleted: bool,
}
