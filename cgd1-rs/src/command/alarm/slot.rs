use serde::Deserialize;
use serde::Serialize;

use super::entry::AlarmEntry;
use super::slot_index::AlarmSlotIndex;

/// An alarm slot read from the device, combining the index and its entry.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct AlarmSlot {
    /// Slot index (0–15).
    pub index: AlarmSlotIndex,
    /// The alarm entry data.
    pub entry: AlarmEntry,
}
