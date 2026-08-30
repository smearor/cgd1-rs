use cgd1_rs::Backend;
use cgd1_rs::MacAddress;

use crate::connection::DeviceConnection;
use crate::error::CliError;

/// Arguments for the `firmware` command.
pub struct FirmwareArgs {
    /// Device MAC address.
    pub address: MacAddress,
    /// BLE backend to use.
    pub backend: Backend,
}

/// Run the `firmware` command.
pub async fn run(args: FirmwareArgs) -> Result<(), CliError> {
    let connection = DeviceConnection::connect(&args.address, args.backend).await?;
    let version = connection.device().read_firmware().await?;

    println!("Firmware: {version}");
    Ok(())
}
