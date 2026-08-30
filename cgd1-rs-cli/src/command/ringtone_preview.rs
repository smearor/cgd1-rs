use cgd1_rs::MacAddress;
use cgd1_rs::Volume;

use crate::connection::DeviceConnection;
use crate::error::CliError;

/// Arguments for the `ringtone-preview` command.
pub struct RingtonePreviewArgs {
    /// Device MAC address.
    pub address: MacAddress,
    /// Volume level (optional, uses device volume if omitted).
    pub volume: Option<Volume>,
}

/// Run the `ringtone-preview` command.
pub async fn run(args: RingtonePreviewArgs) -> Result<(), CliError> {
    let connection = DeviceConnection::connect(&args.address).await?;
    connection.device().preview_ringtone(args.volume).await?;

    println!("Ringtone preview triggered.");
    Ok(())
}
