//! Integration test: exercise WebSocket commands against a virtual CGD1 device.
//!
//! Starts the WS server with `Backend::Virtual`, connects via `tokio-tungstenite`,
//! and sends WebSocket commands: scan → connect → sync_time → read_firmware →
//! read_battery → read_settings. Verifies that each command returns a successful
//! response with the expected payload structure.

use std::time::Duration;

use cgd1_rs::Backend;
use cgd1_rs::FileTokenStore;

use futures::SinkExt;
use futures::StreamExt;
use serde_json::json;
use tokio::net::TcpListener;
use tokio_tungstenite::tungstenite::Message;

/// Default virtual device MAC address.
const VIRTUAL_MAC: &str = "AA:BB:CC:DD:E0:01";

/// Set `XDG_DATA_HOME` to a temp dir so token files don't pollute the user's
/// real data directory.
fn isolate_token_dir() -> std::path::PathBuf {
    let dir = std::env::temp_dir().join(format!(
        "cgd1_ws_test_{}",
        std::process::id()
    ));
    let _ = std::fs::remove_dir_all(&dir);
    std::fs::create_dir_all(&dir).unwrap();
    // SAFETY: Tests are single-threaded; no other code races on XDG_DATA_HOME.
    unsafe { std::env::set_var("XDG_DATA_HOME", &dir) };
    dir
}

/// Pre-generate a token for the virtual device so `connect` succeeds
/// without needing `sync_time` to persist it first.
fn pre_generate_token() {
    let store = FileTokenStore::default_directory();
    let mac: cgd1_rs::MacAddress = VIRTUAL_MAC.parse().unwrap();
    let _ = store.load_or_generate(&mac);
}

/// Send a WS request and read the next response.
async fn send_request(
    ws: &mut tokio_tungstenite::WebSocketStream<tokio_tungstenite::MaybeTlsStream<tokio::net::TcpStream>>,
    id: u32,
    command: serde_json::Value,
) -> serde_json::Value {
    let request = json!({ "id": id, "command": command });
    ws.send(Message::Text(request.to_string().into())).await.expect("send should succeed");

    // Read responses until we find one with our id (skip event pushes).
    loop {
        let msg = ws
            .next()
            .await
            .expect("should receive a response")
            .expect("response should be Ok");
        match msg {
            Message::Text(text) => {
                let response: serde_json::Value = serde_json::from_str(&text).expect("valid JSON");
                if response.get("id").and_then(|v| v.as_u64()) == Some(id as u64) {
                    return response;
                }
            }
            _ => {}
        }
    }
}

/// Assert that a response has no error and has a result field.
fn assert_ok(response: &serde_json::Value, id: u32) -> &serde_json::Value {
    assert_eq!(
        response["id"].as_u64(),
        Some(id as u64),
        "response id should match request id"
    );
    assert!(
        response.get("error").is_none() || response["error"].is_null(),
        "response should not have an error: {response}"
    );
    response
        .get("result")
        .expect("response should have a result field")
}

#[tokio::test]
async fn ws_full_virtual_device_flow() {
    let _token_dir = isolate_token_dir();
    pre_generate_token();

    // Build server state with virtual backend.
    let state = cgd1_rs_ws::ServerState::new(Backend::Virtual)
        .await
        .expect("server state creation should succeed");
    let router = cgd1_rs_ws::build_router(state);

    // Bind to a random port.
    let listener = TcpListener::bind("127.0.0.1:0").await.expect("bind should succeed");
    let addr = listener.local_addr().expect("local addr should be available");

    // Start the server in the background.
    let server_handle = tokio::spawn(async move {
        axum::serve(listener, router).await.expect("server should run");
    });

    // Give the server a moment to start.
    tokio::time::sleep(Duration::from_millis(50)).await;

    // Connect via WebSocket.
    let ws_url = format!("ws://{addr}/ws");
    let (mut ws, _) = tokio_tungstenite::connect_async(&ws_url)
        .await
        .expect("WebSocket connection should succeed");

    // 1. Scan
    let scan_resp = tokio::time::timeout(
        Duration::from_secs(5),
        send_request(&mut ws, 1, json!({ "type": "scan", "duration_secs": 1 })),
    )
    .await
    .expect("scan should not time out");
    let scan_result = assert_ok(&scan_resp, 1);
    assert!(
        scan_result.as_array().map(|a| !a.is_empty()).unwrap_or(false),
        "scan should find at least one device"
    );

    // 2. Connect
    let connect_resp = tokio::time::timeout(
        Duration::from_secs(5),
        send_request(&mut ws, 2, json!({ "type": "connect", "address": VIRTUAL_MAC })),
    )
    .await
    .expect("connect should not time out");
    let connect_result = assert_ok(&connect_resp, 2);
    assert_eq!(
        connect_result["connected"].as_bool(),
        Some(true),
        "device should be connected"
    );

    // 3. SyncTime
    let sync_resp = tokio::time::timeout(
        Duration::from_secs(5),
        send_request(&mut ws, 3, json!({ "type": "sync_time", "address": VIRTUAL_MAC })),
    )
    .await
    .expect("sync_time should not time out");
    let sync_result = assert_ok(&sync_resp, 3);
    assert_eq!(
        sync_result["synced"].as_bool(),
        Some(true),
        "time should be synced"
    );

    // 4. ReadFirmware
    let firmware_resp = tokio::time::timeout(
        Duration::from_secs(5),
        send_request(&mut ws, 4, json!({ "type": "read_firmware", "address": VIRTUAL_MAC })),
    )
    .await
    .expect("read_firmware should not time out");
    let firmware_result = assert_ok(&firmware_resp, 4);
    assert!(
        firmware_result["firmware"].as_str().is_some(),
        "firmware response should contain a firmware string"
    );

    // 5. ReadBattery
    let battery_resp = tokio::time::timeout(
        Duration::from_secs(5),
        send_request(&mut ws, 5, json!({ "type": "read_battery", "address": VIRTUAL_MAC })),
    )
    .await
    .expect("read_battery should not time out");
    let battery_result = assert_ok(&battery_resp, 5);
    assert!(
        battery_result["battery"].as_u64().is_some(),
        "battery response should contain a battery level"
    );

    // 6. ReadSettings
    let settings_resp = tokio::time::timeout(
        Duration::from_secs(5),
        send_request(&mut ws, 6, json!({ "type": "read_settings", "address": VIRTUAL_MAC })),
    )
    .await
    .expect("read_settings should not time out");
    let settings_result = assert_ok(&settings_resp, 6);
    assert!(
        settings_result.get("volume").is_some(),
        "settings response should contain volume"
    );

    // Clean up.
    ws.close(None).await.ok();
    server_handle.abort();

    let _ = std::fs::remove_dir_all(&_token_dir);
}
