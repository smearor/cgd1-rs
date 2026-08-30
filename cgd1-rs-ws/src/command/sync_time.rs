use cgd1_rs::MacAddress;

use crate::error::ServerResult;
use crate::protocol::SyncTimeResponse;
use crate::state::ServerState;

/// Synchronize device time to the current system time.
pub async fn sync_time(state: &ServerState, address: MacAddress) -> ServerResult<SyncTimeResponse> {
    let device = state.device(&address).await?;
    device.sync_time_now().await?;
    Ok(SyncTimeResponse { address, synced: true })
}
