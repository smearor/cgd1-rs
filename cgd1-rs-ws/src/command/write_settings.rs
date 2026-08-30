use cgd1_rs::DeviceSettings;
use cgd1_rs::MacAddress;

use crate::error::ServerResult;
use crate::protocol::WriteSettingsResponse;
use crate::state::ServerState;

/// Write device settings.
pub async fn write_settings(state: &ServerState, address: MacAddress, settings: DeviceSettings) -> ServerResult<WriteSettingsResponse> {
    let device = state.device(&address).await?;
    device.write_settings(&settings).await?;
    Ok(WriteSettingsResponse { address, written: true })
}
