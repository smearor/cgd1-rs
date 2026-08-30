use cgd1_rs::AlarmSlot;
use cgd1_rs::MacAddress;

use crate::error::ServerResult;
use crate::state::ServerState;

/// Read all alarm slots from a device.
pub async fn alarm_list(state: &ServerState, address: MacAddress) -> ServerResult<Vec<AlarmSlot>> {
    let device = state.device(&address).await?;
    let slots = device.read_alarms().await?;
    Ok(slots)
}
