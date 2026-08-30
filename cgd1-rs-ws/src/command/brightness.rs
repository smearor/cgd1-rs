use cgd1_rs::Brightness;
use cgd1_rs::MacAddress;

use crate::error::ServerResult;
use crate::protocol::BrightnessResponse;
use crate::state::ServerState;

/// Set brightness preview on a device.
pub async fn set_brightness(state: &ServerState, address: MacAddress, value: Brightness) -> ServerResult<BrightnessResponse> {
    let device = state.device(&address).await?;
    device.set_brightness(value).await?;
    Ok(BrightnessResponse { address, brightness: value })
}
