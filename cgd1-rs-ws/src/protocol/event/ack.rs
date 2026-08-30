use cgd1_rs::AckStatus;
use cgd1_rs::CommandId;

use serde::Serialize;

/// Event payload: ACK from device.
#[derive(Debug, Clone, Serialize)]
pub struct AckPayload {
    /// Command byte that was acknowledged.
    pub command: CommandId,
    /// Status byte from the device.
    pub status: AckStatus,
}
