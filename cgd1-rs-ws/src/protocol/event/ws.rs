use serde::Serialize;

/// Type of event pushed to subscribed WebSocket clients.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum EventType {
    /// Sensor data update (temperature, humidity).
    SensorUpdate,
    /// Battery level change.
    BatteryLevel,
    /// Device disconnected.
    Disconnected,
    /// Device reconnected.
    Reconnected,
    /// ACK from device.
    Ack,
    /// Passive BLE advertisement received.
    Advertisement,
}

/// Event pushed to subscribed WebSocket clients.
#[derive(Debug, Clone, Serialize)]
pub struct WsEvent<T: Serialize> {
    /// Event type.
    pub event: EventType,
    /// Event payload.
    pub data: T,
}

impl<T: Serialize> WsEvent<T> {
    /// Create a new event.
    pub fn new(event: EventType, data: T) -> Self {
        Self { event, data }
    }
}
