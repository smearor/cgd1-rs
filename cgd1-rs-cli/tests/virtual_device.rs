//! Integration test: spawn the `cgd1` CLI binary with `--backend virtual`
//! and verify its stdout output for each command.

use std::env::temp_dir;
use std::fs::create_dir_all;
use std::fs::remove_dir_all;
use std::path::Path;
use std::path::PathBuf;
use std::process::Command;
use std::time::SystemTime;
use std::time::UNIX_EPOCH;

/// Default virtual device MAC address (first in `DEFAULT_VIRTUAL_MACS`).
const VIRTUAL_MAC: &str = "AA:BB:CC:DD:E0:01";

/// Path to the built CLI binary.
fn cli_binary() -> PathBuf {
    let manifest_dir = env!("CARGO_MANIFEST_DIR");
    let target_dir = std::env::var("CARGO_TARGET_DIR").unwrap_or_else(|_| format!("{manifest_dir}/../target"));
    PathBuf::from(target_dir).join("debug").join("cgd1")
}

/// Set `XDG_DATA_HOME` to a temp dir so token files don't pollute the user's
/// real data directory.
fn isolate_token_dir() -> PathBuf {
    let dir = temp_dir().join(format!(
        "cgd1_cli_test_{}_{}",
        std::process::id(),
        SystemTime::now().duration_since(UNIX_EPOCH).unwrap().as_millis()
    ));
    let _ = remove_dir_all(&dir);
    create_dir_all(&dir).unwrap();
    dir
}

/// Run the CLI binary with `--backend virtual` and the given subcommand args.
/// Returns (exit_status, stdout, stderr).
fn run_cli(args: &[&str], xdg_data_home: &Path) -> (bool, String, String) {
    let binary = cli_binary();
    let output = Command::new(&binary)
        .args(["--backend", "virtual"])
        .args(args)
        .env("XDG_DATA_HOME", xdg_data_home)
        .output()
        .expect("failed to spawn cgd1 CLI binary");

    let stdout = String::from_utf8_lossy(&output.stdout).to_string();
    let stderr = String::from_utf8_lossy(&output.stderr).to_string();
    (output.status.success(), stdout, stderr)
}

/// Pre-generate a token for the virtual device so `connect` succeeds
/// without needing `sync_time` to persist it first.
fn pre_generate_token(xdg_data_home: &Path) {
    let token_dir = xdg_data_home.join("cgd1-rs");
    create_dir_all(&token_dir).unwrap();
    // The token file name is the MAC with colons replaced by underscores.
    let token_file = token_dir.join(VIRTUAL_MAC.replace(':', "_"));
    // Write a 16-byte zero token — the virtual device accepts any token.
    std::fs::write(&token_file, [0u8; 16]).unwrap();
}

#[test]
fn cli_scan_finds_virtual_devices() {
    let token_dir = isolate_token_dir();
    let (success, stdout, stderr) = run_cli(&["scan", "-d", "1"], &token_dir);
    let _ = remove_dir_all(&token_dir);

    assert!(success, "scan should exit 0, stderr: {stderr}");
    assert!(stdout.contains("Found"), "stdout should list found devices: {stdout}");
    assert!(stdout.to_lowercase().contains(&VIRTUAL_MAC.to_lowercase()), "stdout should contain {VIRTUAL_MAC}: {stdout}");
}

#[test]
fn cli_full_virtual_device_flow() {
    let token_dir = isolate_token_dir();
    pre_generate_token(&token_dir);

    // 1. sync-time
    let (ok, stdout, stderr) = run_cli(&["sync-time", VIRTUAL_MAC], &token_dir);
    assert!(ok, "sync-time should exit 0, stderr: {stderr}");
    assert!(stdout.contains("synchronized"), "stdout should mention synchronization: {stdout}");

    // 2. firmware
    let (ok, stdout, stderr) = run_cli(&["firmware", VIRTUAL_MAC], &token_dir);
    assert!(ok, "firmware should exit 0, stderr: {stderr}");
    assert!(stdout.contains("Firmware:"), "stdout should contain firmware version: {stdout}");

    // 3. battery
    let (ok, stdout, stderr) = run_cli(&["battery", VIRTUAL_MAC], &token_dir);
    assert!(ok, "battery should exit 0, stderr: {stderr}");
    assert!(stdout.contains("Battery:"), "stdout should contain battery level: {stdout}");

    // 4. settings-read
    let (ok, stdout, stderr) = run_cli(&["settings-read", VIRTUAL_MAC], &token_dir);
    assert!(ok, "settings-read should exit 0, stderr: {stderr}");
    assert!(stdout.contains("Volume:"), "stdout should contain volume: {stdout}");
    assert!(stdout.contains("Brightness:"), "stdout should contain brightness: {stdout}");

    // 5. alarm-list
    let (ok, stdout, stderr) = run_cli(&["alarm-list", VIRTUAL_MAC], &token_dir);
    assert!(ok, "alarm-list should exit 0, stderr: {stderr}");
    // alarm-list prints either "No alarms set." or a table with "Slot".
    assert!(stdout.contains("Slot") || stdout.contains("No alarms"), "stdout should contain alarm info: {stdout}");

    // 6. brightness
    let (ok, stdout, stderr) = run_cli(&["brightness", VIRTUAL_MAC, "80"], &token_dir);
    assert!(ok, "brightness should exit 0, stderr: {stderr}");
    assert!(stdout.contains("Brightness set to"), "stdout should confirm brightness set: {stdout}");

    let _ = std::fs::remove_dir_all(&token_dir);
}
