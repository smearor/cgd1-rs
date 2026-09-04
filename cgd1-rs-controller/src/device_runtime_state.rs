use std::time::Instant;

/// Per-device runtime state tracked by the controller.
///
/// Each connected device has its own `DeviceRuntimeState` entry, allowing
/// the UI to switch between devices without losing sensor data.
#[derive(Clone)]
pub struct DeviceRuntimeState {
    /// Whether the device is currently connected.
    pub connected: bool,
    /// Timestamp of the last sensor or battery data received from this device.
    pub last_data_time: Option<Instant>,
    /// Last known temperature in degrees Celsius.
    pub temperature: Option<f64>,
    /// Last known humidity percentage.
    pub humidity: Option<f64>,
    /// Last known battery level percentage (0–100).
    pub battery_level: Option<u8>,
}

impl DeviceRuntimeState {
    pub fn new() -> Self {
        Self {
            connected: true,
            last_data_time: Some(Instant::now()),
            temperature: None,
            humidity: None,
            battery_level: None,
        }
    }
}
