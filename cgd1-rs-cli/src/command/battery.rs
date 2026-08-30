use cgd1_rs::Backend;
use cgd1_rs::MacAddress;

use crate::connection::DeviceConnection;
use crate::error::CliError;

/// Arguments for the `battery` command.
pub struct BatteryArgs {
    /// Device MAC address.
    pub address: MacAddress,
    /// BLE backend to use.
    pub backend: Backend,
}

/// Run the `battery` command.
pub async fn run(args: BatteryArgs) -> Result<(), CliError> {
    let connection = DeviceConnection::connect(&args.address, args.backend).await?;
    let level = connection.device().read_battery().await?;

    println!("Battery: {} %", level.value());
    Ok(())
}
