use cgd1_rs::AlarmSlotIndex;
use cgd1_rs::MacAddress;

use crate::error::ServerResult;
use crate::protocol::AlarmDeleteResponse;
use crate::state::ServerState;

/// Delete an alarm at the given slot index.
pub async fn alarm_delete(state: &ServerState, address: MacAddress, slot: AlarmSlotIndex) -> ServerResult<AlarmDeleteResponse> {
    let device = state.device(&address).await?;
    device.delete_alarm(slot).await?;

    Ok(AlarmDeleteResponse { address, slot, deleted: true })
}
