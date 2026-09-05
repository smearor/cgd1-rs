use cgd1_rs::AlarmSlotIndex;
use serde::Serialize;

/// Event payload: alarm triggered on the device.
#[derive(Debug, Clone, Serialize)]
pub struct AlarmTriggeredPayload {
    /// Index of the alarm slot that fired (0-15).
    pub slot: AlarmSlotIndex,
}
