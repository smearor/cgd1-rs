use cgd1_rs::AlarmSlotIndex;
use cgd1_rs::Backend;
use cgd1_rs::MacAddress;

use crate::connection::DeviceConnection;
use crate::error::CliError;

/// Arguments for the `alarm-delete` command.
pub struct AlarmDeleteArgs {
    /// Device MAC address.
    pub address: MacAddress,
    /// Slot index (0–15).
    pub slot: AlarmSlotIndex,
    /// BLE backend to use.
    pub backend: Backend,
}

/// Run the `alarm-delete` command.
pub async fn run(args: AlarmDeleteArgs) -> Result<(), CliError> {
    let connection = DeviceConnection::connect(&args.address, args.backend).await?;
    connection.device().delete_alarm(args.slot).await?;

    println!("Alarm at slot {} deleted.", args.slot);
    Ok(())
}
