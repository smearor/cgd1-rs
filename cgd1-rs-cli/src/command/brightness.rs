use cgd1_rs::Backend;
use cgd1_rs::Brightness;
use cgd1_rs::MacAddress;

use crate::connection::DeviceConnection;
use crate::error::CliError;

/// Arguments for the `brightness` command.
pub struct BrightnessArgs {
    /// Device MAC address.
    pub address: MacAddress,
    /// Brightness value (0–150, multiple of 10).
    pub value: Brightness,
    /// BLE backend to use.
    pub backend: Backend,
}

/// Run the `brightness` command.
pub async fn run(args: BrightnessArgs) -> Result<(), CliError> {
    let connection = DeviceConnection::connect(&args.address, args.backend).await?;
    connection.device().set_brightness(args.value).await?;

    println!("Brightness set to {}.", args.value);
    Ok(())
}
