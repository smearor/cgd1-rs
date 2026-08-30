use cgd1_rs::ClockEvent;
use serde::Serialize;
use serde_json::Value;

use crate::protocol::AckPayload;
use crate::protocol::BatteryLevelPayload;
use crate::protocol::EmptyPayload;
use crate::protocol::EventType;
use crate::protocol::SensorUpdatePayload;
use crate::protocol::WsEvent;

/// Serialize a payload to `serde_json::Value`, returning `None` on failure.
fn to_value(payload: impl Serialize) -> Option<Value> {
    serde_json::to_value(payload).ok()
}

/// Convert a `ClockEvent` to a typed `WsEvent` for WebSocket push.
pub(crate) fn convert_event(event: &ClockEvent) -> Option<WsEvent<Value>> {
    let ws_event: WsEvent<_> = match event {
        ClockEvent::SensorUpdate { temperature, humidity } => WsEvent::new(
            EventType::SensorUpdate,
            to_value(SensorUpdatePayload {
                temperature: *temperature,
                humidity: *humidity,
            })?,
        ),
        ClockEvent::BatteryLevel { level } => WsEvent::new(EventType::BatteryLevel, to_value(BatteryLevelPayload { level: *level })?),
        ClockEvent::Disconnected => WsEvent::new(EventType::Disconnected, to_value(EmptyPayload {})?),
        ClockEvent::Reconnected => WsEvent::new(EventType::Reconnected, to_value(EmptyPayload {})?),
        ClockEvent::Ack { command, status } => WsEvent::new(
            EventType::Ack,
            to_value(AckPayload {
                command: *command,
                status: *status,
            })?,
        ),
        ClockEvent::Advertisement(data) => WsEvent::new(EventType::Advertisement, to_value(data)?),
    };
    Some(ws_event)
}
