use std::sync::Arc;

use axum::extract::State;
use axum::extract::ws::Message;
use axum::extract::ws::WebSocket;
use axum::extract::ws::WebSocketUpgrade;
use axum::response::IntoResponse;
use futures::StreamExt;
use tokio::sync::Mutex;

use crate::protocol::WsRequest;
use crate::protocol::WsResponse;
use crate::session::ws::WsSender;
use crate::state::ServerState;

mod dispatch;
mod event;
mod ws;

/// Handle a WebSocket upgrade request.
pub async fn ws_handler(ws: WebSocketUpgrade, State(state): State<ServerState>) -> impl IntoResponse {
    ws.on_upgrade(|socket| WsSession::run(socket, state))
}

/// A WebSocket session that processes requests and sends responses.
struct WsSession {
    sender: WsSender,
    state: ServerState,
}

impl WsSession {
    /// Run the WebSocket session loop.
    async fn run(socket: WebSocket, state: ServerState) {
        let (sender, mut receiver) = socket.split();
        let sender = Arc::new(Mutex::new(sender));

        let session = WsSession { sender, state };

        while let Some(msg) = receiver.next().await {
            match msg {
                Ok(Message::Text(text)) => {
                    let session = session.clone();
                    tokio::spawn(async move {
                        let response = session.handle_message(&text).await;
                        session.send_response(response).await;
                    });
                }
                Ok(Message::Close(_)) | Err(_) => break,
                _ => {}
            }
        }
    }

    /// Handle a single WebSocket message and return a response.
    async fn handle_message(&self, text: &str) -> WsResponse {
        let request: WsRequest = match serde_json::from_str(text) {
            Ok(req) => req,
            Err(e) => {
                return WsResponse::error(0, format!("invalid JSON: {e}"));
            }
        };

        let id = request.id;
        let result = self.dispatch_command(request.command).await;

        match result {
            Ok(value) => WsResponse::ok(id, &value),
            Err(e) => WsResponse::error(id, e.to_string()),
        }
    }
}

impl Clone for WsSession {
    fn clone(&self) -> Self {
        Self {
            sender: self.sender.clone(),
            state: self.state.clone(),
        }
    }
}
