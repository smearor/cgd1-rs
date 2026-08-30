use cgd1_rs::MacAddress;

use crate::error::ServerResult;
use crate::protocol::FirmwareResponse;
use crate::state::ServerState;

/// Read firmware version from a device.
pub async fn read_firmware(state: &ServerState, address: MacAddress) -> ServerResult<FirmwareResponse> {
    let device = state.device(&address).await?;
    let firmware = device.read_firmware().await?;
    Ok(FirmwareResponse { address, firmware })
}
