use cgd1_rs::Backend;
use cgd1_rs::ClockDevice;
use cgd1_rs::ClockEvent;
use cgd1_rs::MacAddress;
use std::time::Duration;
use tokio::sync::broadcast::error::RecvError;
use tokio::time::Instant;
use tokio::time::sleep_until;
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
    /// BLE backend to use.
    pub backend: Backend,
}

/// Run the `monitor` command.
pub async fn run(args: MonitorArgs) -> Result<(), CliError> {
    let connection = DeviceConnection::connect(&args.address, args.backend).await?;
    run_monitor(connection.device(), args.duration).await
}

/// Monitor sensor data from an already-connected device.
pub async fn run_monitor(device: &ClockDevice, duration: MonitorDuration) -> Result<(), CliError> {
    let mut receiver = device.subscribe();

    println!("Monitoring sensor data (Ctrl+C to stop)...");

    let deadline = if duration.is_indefinite() {
        None
    } else {
        Some(Instant::now() + Duration::from_secs(duration.seconds()))
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
                    Err(RecvError::Lagged(n)) => {
                        warn!("Skipped {n} events.");
                    }
                    Err(_) => break,
                }
            }
            _ = async {
                if let Some(dl) = deadline {
                    sleep_until(dl).await;
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
