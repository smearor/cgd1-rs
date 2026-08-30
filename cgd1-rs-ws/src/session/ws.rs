use std::sync::Arc;

use axum::extract::ws::Message;
use axum::extract::ws::WebSocket;
use futures::SinkExt;
use futures::stream::SplitSink;
use tokio::sync::Mutex;
use tracing::warn;

use crate::protocol::WsResponse;
use crate::session::WsSession;

/// Type alias for the WebSocket sender half.
pub(crate) type WsSender = Arc<Mutex<SplitSink<WebSocket, Message>>>;

impl WsSession {
    /// Send a WebSocket response to the client.
    pub(crate) async fn send_response(&self, response: WsResponse) {
        let json = serde_json::to_string(&response).unwrap_or_else(|e| format!("{{\"id\":{},\"error\":\"serialization failed: {}\"}}", response.id, e));
        let mut tx = self.sender.lock().await;
        if tx.send(Message::Text(json.into())).await.is_err() {
            warn!("Failed to send WebSocket response");
        }
    }
}
