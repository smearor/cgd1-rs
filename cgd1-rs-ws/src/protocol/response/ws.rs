use serde::Serialize;

/// Outgoing WebSocket response.
#[derive(Debug, Clone, Serialize)]
pub struct WsResponse {
    /// Request ID that this response corresponds to.
    pub id: u32,
    /// Response payload.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub result: Option<serde_json::Value>,
    /// Error message if the command failed.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub error: Option<String>,
}

impl WsResponse {
    /// Create a successful response from a serializable value.
    pub fn ok(id: u32, result: &impl Serialize) -> Self {
        Self {
            id,
            result: Some(serde_json::to_value(result).unwrap_or_else(|e| serde_json::json!({ "error": format!("serialization failed: {e}") }))),
            error: None,
        }
    }

    /// Create an error response.
    pub fn error(id: u32, message: impl Into<String>) -> Self {
        Self {
            id,
            result: None,
            error: Some(message.into()),
        }
    }
}
