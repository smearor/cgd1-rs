use cgd1_rs::AlarmEntry;
use cgd1_rs::AlarmSlotIndex;
use cgd1_rs::ClockTime;
use cgd1_rs::DayMask;
use cgd1_rs::MacAddress;

use crate::error::ServerResult;
use crate::protocol::AlarmSetResponse;
use crate::state::ServerState;

/// Set an alarm at the given slot index.
pub async fn alarm_set(
    state: &ServerState,
    address: MacAddress,
    slot: AlarmSlotIndex,
    time: ClockTime,
    repeat_mask: DayMask,
    enabled: bool,
    snooze: bool,
) -> ServerResult<AlarmSetResponse> {
    let device = state.device(&address).await?;

    let entry = AlarmEntry::new(time, repeat_mask, enabled, snooze);
    device.set_alarm(&entry, slot).await?;

    Ok(AlarmSetResponse { address, slot, set: true })
}
