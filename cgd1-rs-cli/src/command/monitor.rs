use std::time::Duration;

use cgd1_rs::ClockEvent;
use cgd1_rs::MacAddress;
use tracing::debug;
use tracing::info;
use tracing::warn;

use crate::connection::DeviceConnection;
use crate::error::CliError;
use crate::monitor_duration::MonitorDuration;

/// Arguments for the `monitor` command.
pub struct MonitorArgs {
    /// Device MAC address.
    pub address: MacAddress,
    /// Duration (0 = indefinite).
    pub duration: MonitorDuration,
}

/// Run the `monitor` command.
pub async fn run(args: MonitorArgs) -> Result<(), CliError> {
    let connection = DeviceConnection::connect(&args.address).await?;
    let mut receiver = connection.device().subscribe();

    println!("Monitoring sensor data (Ctrl+C to stop)...");

    let deadline = if args.duration.is_indefinite() {
        None
    } else {
        Some(tokio::time::Instant::now() + Duration::from_secs(args.duration.seconds()))
    };

    loop {
        tokio::select! {
            event = receiver.recv() => {
                match event {
                    Ok(ClockEvent::SensorUpdate { temperature, humidity }) => {
                        println!("{:.1} C  {:.1} %", temperature.value(), humidity.value());
                    }
                    Ok(ClockEvent::BatteryLevel { level }) => {
                        println!("Battery: {} %", level.value());
                    }
                    Ok(ClockEvent::Disconnected) => {
                        println!("Device disconnected.");
                        break;
                    }
                    Ok(ClockEvent::Ack { command, status }) => {
                        debug!(?command, ?status, "ACK event");
                    }
                    Ok(ClockEvent::Reconnected) => {
                        info!("Device reconnected.");
                    }
                    Ok(ClockEvent::Advertisement(_)) => {}
                    Err(tokio::sync::broadcast::error::RecvError::Lagged(n)) => {
                        warn!("Skipped {n} events.");
                    }
                    Err(_) => break,
                }
            }
            _ = async {
                if let Some(dl) = deadline {
                    tokio::time::sleep_until(dl).await;
                } else {
                    std::future::pending::<()>().await;
                }
            } => {
                println!("Monitoring duration elapsed.");
                break;
            }
        }
    }

    Ok(())
}
