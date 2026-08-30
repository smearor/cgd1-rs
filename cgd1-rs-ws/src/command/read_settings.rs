use cgd1_rs::DeviceSettings;
use cgd1_rs::MacAddress;

use crate::error::ServerResult;
use crate::state::ServerState;

/// Read device settings.
pub async fn read_settings(state: &ServerState, address: MacAddress) -> ServerResult<DeviceSettings> {
    let device = state.device(&address).await?;
    let settings = device.read_settings().await?;
    Ok(settings)
}
