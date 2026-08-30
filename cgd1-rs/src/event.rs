use crate::AdvertisementData;
use crate::command::AckStatus;
use crate::command::CommandId;
use crate::types::BatteryLevel;
use crate::types::Humidity;
use crate::types::Temperature;

use serde::Deserialize;
use serde::Serialize;

/// Events emitted by a connected CGD1 device.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ClockEvent {
    /// Real-time sensor update (temperature, humidity).
    SensorUpdate {
        /// Temperature in degrees Celsius.
        temperature: Temperature,
        /// Relative humidity in percent.
        humidity: Humidity,
    },
    /// Battery level update.
    BatteryLevel {
        /// Battery percentage (0-100).
        level: BatteryLevel,
    },
    /// ACK received for a command.
    Ack {
        /// Command ID that was acknowledged.
        command: CommandId,
        /// Status of the acknowledged command.
        status: AckStatus,
    },
    /// Device disconnected.
    Disconnected,
    /// Device reconnected and state recovered (auth + subscriptions restored).
    /// Callers that queued commands during the outage can retry.
    Reconnected,
    /// Passive advertisement received (no connection needed).
    Advertisement(AdvertisementData),
}
