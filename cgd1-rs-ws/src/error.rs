use axum::Json;
use axum::http::StatusCode;
use axum::response::IntoResponse;
use axum::response::Response;
use cgd1_rs::ClockError;
use cgd1_rs::MacAddress;
use serde::Serialize;
use thiserror::Error;

/// Errors returned by the WebSocket server.
#[derive(Debug, Error)]
pub enum ServerError {
    /// A core library error.
    #[error(transparent)]
    Core(#[from] ClockError),

    /// Device is not connected.
    #[error("device {address} is not connected")]
    NotConnected {
        /// The MAC address of the device.
        address: MacAddress,
    },

    /// Failed to serialize or deserialize JSON.
    #[error("JSON error: {0}")]
    Json(#[from] serde_json::Error),
}

/// JSON error body returned by REST endpoints.
#[derive(Debug, Serialize)]
struct ErrorBody {
    error: String,
}

impl ServerError {
    /// Convert a server error into an HTTP status code.
    pub fn status_code(&self) -> u16 {
        match self {
            Self::NotConnected { .. } => 404,
            Self::Json { .. } => 400,
            Self::Core(_) => 500,
        }
    }
}

impl IntoResponse for ServerError {
    fn into_response(self) -> Response {
        let status = StatusCode::from_u16(self.status_code()).unwrap_or(StatusCode::INTERNAL_SERVER_ERROR);
        let body = ErrorBody { error: self.to_string() };
        (status, Json(body)).into_response()
    }
}

/// Type alias for server results.
pub type ServerResult<T> = Result<T, ServerError>;
