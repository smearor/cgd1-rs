use cgd1_rs::MacAddress;

use crate::error::ServerResult;
use crate::protocol::DisconnectResponse;
use crate::state::ServerState;

/// Disconnect from a device by MAC address.
pub async fn disconnect(state: &ServerState, address: MacAddress) -> ServerResult<DisconnectResponse> {
    state.disconnect(&address).await?;
    Ok(DisconnectResponse { address, disconnected: true })
}
