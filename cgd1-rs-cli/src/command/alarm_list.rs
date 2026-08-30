use cgd1_rs::MacAddress;

use crate::connection::DeviceConnection;
use crate::error::CliError;

/// Arguments for the `alarm-list` command.
pub struct AlarmListArgs {
    /// Device MAC address.
    pub address: MacAddress,
}

/// Run the `alarm-list` command.
pub async fn run(args: AlarmListArgs) -> Result<(), CliError> {
    let connection = DeviceConnection::connect(&args.address).await?;
    let slots = connection.device().read_alarms().await?;

    if slots.is_empty() {
        println!("No alarms set.");
        return Ok(());
    }

    println!("{:<6} {:<8} {:<3} {:<3} {:<6} {:<6}", "Slot", "Enabled", "HH", "MM", "Days", "Snooze");
    for slot in &slots {
        println!(
            "{:<6} {:<8} {:02}   {:02}   0x{:02x}  {}",
            slot.index.value(),
            if slot.entry.enabled() { "yes" } else { "no" },
            slot.entry.hour(),
            slot.entry.minute(),
            slot.entry.repeat_mask().value(),
            if slot.entry.snooze() { "yes" } else { "no" },
        );
    }

    Ok(())
}
