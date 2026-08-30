use serde::Serialize;

/// Event payload: empty (for disconnected/reconnected events).
#[derive(Debug, Clone, Serialize)]
pub struct EmptyPayload {}
