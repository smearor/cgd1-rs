use cgd1_rs::Humidity;
use cgd1_rs::Temperature;

use serde::Serialize;

/// Event payload: sensor update.
#[derive(Debug, Clone, Serialize)]
pub struct SensorUpdatePayload {
    /// Temperature in degrees Celsius.
    pub temperature: Temperature,
    /// Relative humidity percentage.
    pub humidity: Humidity,
}
