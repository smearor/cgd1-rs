use cgd1_rs::MacAddress;

use crate::error::ServerResult;
use crate::protocol::ConnectResponse;
use crate::state::ServerState;

/// Connect to a device by MAC address and authenticate.
pub async fn connect(state: &ServerState, address: MacAddress) -> ServerResult<ConnectResponse> {
    state.connect(&address).await?;
    Ok(ConnectResponse { address, connected: true })
}
