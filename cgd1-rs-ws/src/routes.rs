use axum::Json;
use axum::Router;
use axum::extract::Path;
use axum::extract::State;
use axum::response::Html;
use axum::response::IntoResponse;
use axum::routing::get;
use cgd1_rs::MacAddress;
use serde_json::json;

use crate::error::ServerError;
use crate::session::ws_handler;
use crate::state::ServerState;

/// Build the axum router with all REST and WebSocket routes.
pub fn build_router(state: ServerState) -> Router {
    Router::new()
        .route("/", get(index_page))
        .route("/health", get(health_check))
        .route("/api/devices", get(list_devices))
        .route("/api/devices/{address}/sensors", get(get_sensors))
        .route("/api/devices/{address}/battery", get(get_battery))
        .route("/api/devices/{address}/firmware", get(get_firmware))
        .route("/api/devices/{address}/alarms", get(get_alarms))
        .route("/api/devices/{address}/settings", get(get_settings))
        .route("/ws", get(ws_handler))
        .with_state(state)
}

/// Serve the single-file HTML web app.
async fn index_page() -> impl IntoResponse {
    Html(include_str!("../static/index.html"))
}

/// Health check endpoint.
async fn health_check() -> impl IntoResponse {
    Json(json!({ "status": "ok" }))
}

/// List all connected device addresses.
async fn list_devices(State(state): State<ServerState>) -> impl IntoResponse {
    let addresses = state.manager().connected_devices().await;
    Json(json!({ "devices": addresses }))
}

/// Get latest sensor data for a device.
async fn get_sensors(State(state): State<ServerState>, Path(address): Path<MacAddress>) -> Result<Json<serde_json::Value>, ServerError> {
    let device = state.device(&address).await?;
    let settings = device.read_settings().await?;
    Ok(Json(serde_json::to_value(&settings)?))
}

/// Get battery level for a device.
async fn get_battery(State(state): State<ServerState>, Path(address): Path<MacAddress>) -> Result<Json<serde_json::Value>, ServerError> {
    let device = state.device(&address).await?;
    let battery = device.read_battery().await?;
    Ok(Json(json!({ "address": address, "battery": battery.value() })))
}

/// Get firmware version for a device.
async fn get_firmware(State(state): State<ServerState>, Path(address): Path<MacAddress>) -> Result<Json<serde_json::Value>, ServerError> {
    let device = state.device(&address).await?;
    let firmware = device.read_firmware().await?;
    Ok(Json(json!({ "address": address, "firmware": firmware })))
}

/// Get all alarms for a device.
async fn get_alarms(State(state): State<ServerState>, Path(address): Path<MacAddress>) -> Result<Json<serde_json::Value>, ServerError> {
    let device = state.device(&address).await?;
    let slots = device.read_alarms().await?;
    Ok(Json(serde_json::to_value(&slots)?))
}

/// Get device settings.
async fn get_settings(State(state): State<ServerState>, Path(address): Path<MacAddress>) -> Result<Json<serde_json::Value>, ServerError> {
    let device = state.device(&address).await?;
    let settings = device.read_settings().await?;
    Ok(Json(serde_json::to_value(&settings)?))
}
