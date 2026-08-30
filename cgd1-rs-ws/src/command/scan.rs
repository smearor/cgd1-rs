use std::time::Duration;

use cgd1_rs::DiscoveredDevice;
use cgd1_rs::ScanDuration;

use crate::error::ServerResult;
use crate::state::ServerState;

/// Run the `scan` command.
pub async fn scan(state: &ServerState, duration: ScanDuration) -> ServerResult<Vec<DiscoveredDevice>> {
    let scanner = state.scanner();
    let devices = scanner.scan_active(Duration::from(duration)).await?;
    Ok(devices)
}
