use cgd1_rs::MacAddress;

use crate::connection::DeviceConnection;
use crate::error::CliError;

/// Arguments for the `firmware` command.
pub struct FirmwareArgs {
    /// Device MAC address.
    pub address: MacAddress,
}

/// Run the `firmware` command.
pub async fn run(args: FirmwareArgs) -> Result<(), CliError> {
    let connection = DeviceConnection::connect(&args.address).await?;
    let version = connection.device().read_firmware().await?;

    println!("Firmware: {version}");
    Ok(())
}
