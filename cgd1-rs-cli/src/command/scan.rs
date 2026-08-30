use std::sync::Arc;
use std::time::Duration;

use cgd1_rs::BtleplugTransport;
use cgd1_rs::ClockScanner;
use cgd1_rs::ScanDuration;

use crate::error::CliError;

/// Arguments for the `scan` command.
pub struct ScanArgs {
    /// Scan duration in seconds (1–600).
    pub duration: ScanDuration,
}

/// Run the `scan` command.
pub async fn run(args: ScanArgs) -> Result<(), CliError> {
    let transport = Arc::new(BtleplugTransport::new().await?);
    let scanner = ClockScanner::new(transport);

    println!("Scanning for {}...", args.duration);
    let devices = scanner.scan_active(Duration::from(args.duration)).await?;

    if devices.is_empty() {
        println!("No CGD1 devices found.");
        return Ok(());
    }

    println!("Found {} device(s):", devices.len());
    for device in &devices {
        println!("  MAC: {}", device.address);
        if let Some(ad) = &device.advertisement {
            println!("    Temperature: {:.1} C", ad.temperature.value());
            println!("    Humidity: {:.1} %", ad.humidity.value());
            println!("    Battery: {} %", ad.battery.value());
        }
        if let Some(rssi) = device.rssi {
            println!("    RSSI: {}", rssi.dbm());
        }
    }

    Ok(())
}
