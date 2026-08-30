use serde::Deserialize;

use crate::protocol::command::WsCommand;

/// Incoming WebSocket request.
#[derive(Debug, Clone, Deserialize)]
pub struct WsRequest {
    /// Request ID for matching responses.
    pub id: u32,
    /// Command to execute.
    pub command: WsCommand,
}
