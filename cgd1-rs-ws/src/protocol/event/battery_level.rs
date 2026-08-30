use cgd1_rs::BatteryLevel;

use serde::Serialize;

/// Event payload: battery level.
#[derive(Debug, Clone, Serialize)]
pub struct BatteryLevelPayload {
    /// Battery level percentage (0–100).
    pub level: BatteryLevel,
}
