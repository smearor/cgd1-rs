mod cli;
mod command;
mod connection;
mod error;
mod monitor_duration;
mod repl;

use clap::Parser;
use tracing_subscriber::EnvFilter;

use cli::Cli;
use cli::Commands;
use command::alarm_delete::AlarmDeleteArgs;
use command::alarm_list::AlarmListArgs;
use command::alarm_set::AlarmSetArgs;
use command::battery::BatteryArgs;
use command::brightness::BrightnessArgs;
use command::firmware::FirmwareArgs;
use command::monitor::MonitorArgs;
use command::ringtone_preview::RingtonePreviewArgs;
use command::ringtone_upload::RingtoneUploadArgs;
use command::scan::ScanArgs;
use command::settings_read::SettingsReadArgs;
use command::settings_write::SettingsWriteArgs;
use command::sync_time::SyncTimeArgs;
use error::CliError;

#[tokio::main]
async fn main() -> miette::Result<()> {
    let cli = Cli::parse();

    let filter = match cli.verbose {
        0 => EnvFilter::new("warn"),
        1 => EnvFilter::new("info"),
        2 => EnvFilter::new("debug"),
        _ => EnvFilter::new("trace"),
    };
    tracing_subscriber::fmt().with_env_filter(filter).init();

    run(cli).await.map_err(miette::Report::from)
}

async fn run(cli: Cli) -> Result<(), CliError> {
    let backend = cli.backend;
    match cli.command {
        Commands::Scan { duration } => command::scan::run(ScanArgs { duration, backend }).await,
        Commands::SyncTime { address } => command::sync_time::run(SyncTimeArgs { address, backend }).await,
        Commands::AlarmList { address } => command::alarm_list::run(AlarmListArgs { address, backend }).await,
        Commands::AlarmSet {
            address,
            slot,
            time,
            repeat,
            no_snooze,
        } => {
            command::alarm_set::run(AlarmSetArgs {
                address,
                slot,
                time,
                repeat,
                snooze: !no_snooze,
                backend,
            })
            .await
        }
        Commands::AlarmDelete { address, slot } => command::alarm_delete::run(AlarmDeleteArgs { address, slot, backend }).await,
        Commands::SettingsRead { address } => command::settings_read::run(SettingsReadArgs { address, backend }).await,
        Commands::SettingsWrite {
            address,
            volume,
            brightness,
            night_brightness,
            timezone,
            time_format,
            temp_unit,
            language,
        } => {
            command::settings_write::run(SettingsWriteArgs {
                address,
                volume,
                brightness,
                night_brightness,
                timezone,
                time_format,
                temp_unit,
                language,
                backend,
            })
            .await
        }
        Commands::Brightness { address, value } => command::brightness::run(BrightnessArgs { address, value, backend }).await,
        Commands::RingtonePreview { address, volume } => command::ringtone_preview::run(RingtonePreviewArgs { address, volume, backend }).await,
        Commands::RingtoneUpload { address, file, signature } => {
            command::ringtone_upload::run(RingtoneUploadArgs {
                address,
                file,
                signature,
                backend,
            })
            .await
        }
        Commands::Firmware { address } => command::firmware::run(FirmwareArgs { address, backend }).await,
        Commands::Battery { address } => command::battery::run(BatteryArgs { address, backend }).await,
        Commands::Monitor { address, duration } => command::monitor::run(MonitorArgs { address, duration, backend }).await,
        Commands::Repl { address } => repl::run(backend, address).await,
    }
}
