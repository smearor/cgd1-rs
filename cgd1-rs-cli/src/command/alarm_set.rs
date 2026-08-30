use cgd1_rs::AlarmEntry;
use cgd1_rs::AlarmSlotIndex;
use cgd1_rs::Backend;
use cgd1_rs::ClockTime;
use cgd1_rs::DayMask;
use cgd1_rs::MacAddress;

use crate::connection::DeviceConnection;
use crate::error::CliError;

/// Arguments for the `alarm-set` command.
pub struct AlarmSetArgs {
    /// Device MAC address.
    pub address: MacAddress,
    /// Slot index (0–15).
    pub slot: AlarmSlotIndex,
    /// Alarm time (HH:MM).
    pub time: ClockTime,
    /// Repeat mask as day-of-week bitmask.
    pub repeat: DayMask,
    /// Whether snooze is enabled.
    pub snooze: bool,
    /// BLE backend to use.
    pub backend: Backend,
}

/// Run the `alarm-set` command.
pub async fn run(args: AlarmSetArgs) -> Result<(), CliError> {
    let entry = AlarmEntry::new(args.time, args.repeat, true, args.snooze);

    let connection = DeviceConnection::connect(&args.address, args.backend).await?;
    connection.device().set_alarm(&entry, args.slot).await?;

    println!("Alarm set at slot {} for {} (repeat 0x{:02x}).", args.slot, args.time, args.repeat.value());
    Ok(())
}
