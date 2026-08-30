use cgd1_rs::MacAddress;

use crate::error::ServerResult;
use crate::protocol::BatteryResponse;
use crate::state::ServerState;

/// Read battery level from a device.
pub async fn read_battery(state: &ServerState, address: MacAddress) -> ServerResult<BatteryResponse> {
    let device = state.device(&address).await?;
    let battery = device.read_battery().await?;
    Ok(BatteryResponse { address, battery })
}
