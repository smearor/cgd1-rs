use cgd1_rs::MacAddress;
use cgd1_rs::Volume;

use crate::error::ServerResult;
use crate::protocol::PreviewRingtoneResponse;
use crate::state::ServerState;

/// Preview ringtone at current or specified volume.
pub async fn preview_ringtone(state: &ServerState, address: MacAddress, volume: Option<Volume>) -> ServerResult<PreviewRingtoneResponse> {
    let device = state.device(&address).await?;
    device.preview_ringtone(volume).await?;
    Ok(PreviewRingtoneResponse { address, previewing: true })
}
