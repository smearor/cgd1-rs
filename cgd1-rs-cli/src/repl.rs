use std::sync::Arc;

use cgd1_rs::AlarmEntry;
use cgd1_rs::Backend;
use cgd1_rs::BleTransport;
use cgd1_rs::ClockDevice;
use cgd1_rs::ClockError;
use cgd1_rs::ClockManager;
use cgd1_rs::ClockScanner;
use cgd1_rs::FileTokenStore;
use cgd1_rs::Language;
use cgd1_rs::MacAddress;
use cgd1_rs::ScanDuration;
use cgd1_rs::TemperatureUnit;
use cgd1_rs::TimeFormat;
use cgd1_rs::TokenStore;
use clap::Parser;
use rustyline::DefaultEditor;
use rustyline::error::ReadlineError;
use tracing::info;

use crate::cli::Cli;
use crate::cli::Commands;
use crate::connection::DeviceConnection;
use crate::error::CliError;

/// Persistent REPL session state.
struct ReplState {
    transport: Arc<dyn BleTransport>,
    manager: ClockManager,
    token_store: Arc<FileTokenStore>,
    connection: Option<DeviceConnection>,
}

impl ReplState {
    /// Create a new REPL state with the given backend.
    async fn new(backend: Backend) -> Result<Self, CliError> {
        let transport = backend.create_transport().await?;
        let manager = ClockManager::new(transport.clone());
        let token_store = Arc::new(FileTokenStore::default_directory());
        Ok(Self {
            transport,
            manager,
            token_store,
            connection: None,
        })
    }

    /// Connect to a device by MAC address.
    async fn connect(&mut self, mac: &MacAddress) -> Result<(), CliError> {
        if self.connection.is_some() {
            eprintln!("Already connected. Use `disconnect` first.");
            return Ok(());
        }

        let device = self.manager.connect(mac).await?;
        let token_result = self.token_store.load_or_generate(mac);
        device.set_token_store(self.token_store.clone() as Arc<dyn TokenStore>).await;
        device.authenticate(&token_result).await?;
        self.connection = Some(DeviceConnection::new(self.transport.clone(), device));
        println!("Connected to {mac}.");
        Ok(())
    }

    /// Disconnect from the current device.
    async fn disconnect(&mut self) -> Result<(), CliError> {
        if let Some(conn) = self.connection.take() {
            let mac = *conn.device().address();
            self.manager.disconnect(&mac).await?;
            println!("Disconnected from {mac}.");
        } else {
            println!("Not connected.");
        }
        Ok(())
    }

    /// Get the connected device, or print an error.
    fn device(&self) -> Option<&ClockDevice> {
        self.connection.as_ref().map(|c| c.device())
    }

    /// Run a scan and print results.
    async fn scan(&self, duration: ScanDuration) -> Result<(), CliError> {
        let scanner = ClockScanner::new(self.transport.clone());
        let devices = scanner.scan_active(std::time::Duration::from(duration)).await?;

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
        }
        Ok(())
    }
}

/// Run the REPL loop.
pub async fn run(backend: Backend, initial_address: Option<MacAddress>) -> Result<(), CliError> {
    let mut state = ReplState::new(backend).await?;
    let mut rl = DefaultEditor::new().expect("failed to initialize readline");

    if let Some(mac) = initial_address {
        state.connect(&mac).await?;
    }

    println!("cgd1 REPL — type 'help' for commands, 'exit' to quit.");

    loop {
        let prompt = match state.connection {
            Some(ref conn) => format!("cgd1({})> ", conn.device().address()),
            None => "cgd1> ".to_string(),
        };

        let line = match rl.readline(&prompt) {
            Ok(line) => {
                let trimmed = line.trim();
                if trimmed.is_empty() {
                    continue;
                }
                let _ = rl.add_history_entry(trimmed);
                trimmed.to_string()
            }
            Err(ReadlineError::Interrupted | ReadlineError::Eof) => {
                println!("bye.");
                break;
            }
            Err(e) => {
                eprintln!("readline error: {e}");
                break;
            }
        };

        if let Err(e) = handle_line(&mut state, &line).await {
            eprintln!("error: {e}");
        }
    }

    if let Some(conn) = state.connection.take() {
        let mac = *conn.device().address();
        let _ = state.manager.disconnect(&mac).await;
    }

    Ok(())
}

/// Handle a single REPL input line.
async fn handle_line(state: &mut ReplState, line: &str) -> Result<(), CliError> {
    let tokens = shell_words::split(line).unwrap_or_default();
    if tokens.is_empty() {
        return Ok(());
    }

    match tokens[0].as_str() {
        "exit" | "quit" => {
            std::process::exit(0);
        }
        "help" => {
            print_help();
            return Ok(());
        }
        "connect" => {
            let mac: MacAddress = tokens
                .get(1)
                .ok_or_else(|| CliError::Core(ClockError::Parse("missing MAC address".into())))?
                .parse()
                .map_err(|e| CliError::Core(ClockError::Parse(format!("invalid MAC: {e}"))))?;
            return state.connect(&mac).await;
        }
        "disconnect" => {
            return state.disconnect().await;
        }
        _ => {}
    }

    // Parse as a full CLI command (reuses clap definitions).
    let mut args = vec!["cgd1".to_string()];
    args.extend(tokens.iter().cloned());

    let cli = match Cli::try_parse_from(&args) {
        Ok(cli) => cli,
        Err(e) => {
            e.print().ok();
            return Ok(());
        }
    };

    dispatch(state, cli).await
}

/// Dispatch a parsed CLI command in the REPL context.
async fn dispatch(state: &mut ReplState, cli: Cli) -> Result<(), CliError> {
    match cli.command {
        Commands::Scan { duration } => {
            state.scan(duration).await?;
        }
        Commands::Repl { .. } => {
            eprintln!("nested REPL is not supported.");
        }
        Commands::SyncTime { address } => {
            let device = require_device(state, &address)?;
            device.sync_time_now().await?;
            let token_result = state.token_store.load_or_generate(&address);
            if token_result.is_new() {
                state.token_store.save(&address, &token_result)?;
                info!(address = %address, "new token persisted after successful sync_time");
            }
            println!("Time synchronized for {address}.");
        }
        Commands::Firmware { address } => {
            let device = require_device(state, &address)?;
            let version = device.read_firmware().await?;
            println!("Firmware: {version}");
        }
        Commands::Battery { address } => {
            let device = require_device(state, &address)?;
            let level = device.read_battery().await?;
            println!("Battery: {} %", level.value());
        }
        Commands::SettingsRead { address } => {
            let device = require_device(state, &address)?;
            let settings = device.read_settings().await?;
            println!("Volume: {}", settings.volume());
            println!("Brightness: {}", settings.brightness().nibble() * 10);
            println!("Night brightness: {}", settings.night_brightness().nibble() * 10);
            println!("Time format: {}", if settings.time_format() == TimeFormat::TwelveHour { "12h" } else { "24h" });
            println!(
                "Temperature unit: {}",
                if settings.temperature_unit() == TemperatureUnit::Fahrenheit {
                    "F"
                } else {
                    "C"
                }
            );
            println!("Language: {}", if settings.language() == Language::English { "en" } else { "zh" });
            println!("Timezone: {:+} min", settings.timezone().minutes());
            println!("Screen light duration: {}", settings.screen_light_duration());
            println!(
                "Night mode: {} ({}–{})",
                if settings.night_mode_enabled() { "on" } else { "off" },
                settings.night_start(),
                settings.night_end(),
            );
            println!("Ringtone signature: {}", settings.ringtone_signature());
        }
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
            let device = require_device(state, &address)?;
            let mut settings = device.read_settings().await?;
            if let Some(vol) = volume {
                settings = settings.with_volume(vol)?;
            }
            if let Some(b) = brightness {
                settings = settings.with_brightness(b)?;
            }
            if let Some(nb) = night_brightness {
                settings = settings.with_night_brightness(nb)?;
            }
            if let Some(tz) = timezone {
                settings = settings.with_timezone(tz)?;
            }
            if let Some(tf) = time_format {
                settings = settings.with_time_format(tf);
            }
            if let Some(tu) = temp_unit {
                settings = settings.with_temperature_unit(tu);
            }
            if let Some(lang) = language {
                settings = settings.with_language(lang);
            }
            device.write_settings(&settings).await?;
            println!("Settings updated.");
        }
        Commands::AlarmList { address } => {
            let device = require_device(state, &address)?;
            let slots = device.read_alarms().await?;
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
        }
        Commands::AlarmSet {
            address,
            slot,
            time,
            repeat,
            no_snooze,
        } => {
            let device = require_device(state, &address)?;
            let entry = AlarmEntry::new(time, repeat, true, !no_snooze);
            device.set_alarm(&entry, slot).await?;
            println!("Alarm set at slot {}.", slot.value());
        }
        Commands::AlarmDelete { address, slot } => {
            let device = require_device(state, &address)?;
            device.delete_alarm(slot).await?;
            println!("Alarm deleted at slot {}.", slot.value());
        }
        Commands::Brightness { address, value } => {
            let device = require_device(state, &address)?;
            device.set_brightness(value).await?;
            println!("Brightness set to {}.", value);
        }
        Commands::RingtonePreview { address, volume } => {
            let device = require_device(state, &address)?;
            device.preview_ringtone(volume).await?;
            println!("Ringtone preview started.");
        }
        Commands::RingtoneUpload { address, file, signature } => {
            let device = require_device(state, &address)?;
            let data = std::fs::read(&file).map_err(|e| CliError::AudioReadFailed {
                path: file.clone(),
                reason: e.to_string(),
            })?;
            device.upload_ringtone(&data, signature.bytes()).await?;
            println!("Ringtone uploaded: {signature}");
        }
        Commands::Monitor { address, duration } => {
            let device = require_device(state, &address)?;
            crate::command::monitor::run_monitor(device, duration).await?;
        }
    }
    Ok(())
}

/// Get the connected device for the given address, or error.
fn require_device<'a>(state: &'a ReplState, address: &MacAddress) -> Result<&'a ClockDevice, CliError> {
    match state.device() {
        Some(device) if device.address() == address => Ok(device),
        Some(device) => Err(CliError::Core(ClockError::Parse(format!("connected to {}, but command targets {address}", device.address())))),
        None => Err(CliError::Core(ClockError::Parse("not connected. Use `connect <mac>` first.".into()))),
    }
}

/// Print REPL help.
fn print_help() {
    println!("REPL commands:");
    println!("  connect <mac>          Connect to a device");
    println!("  disconnect             Disconnect from current device");
    println!("  scan [-d <secs>]       Scan for devices");
    println!("  exit | quit            Exit the REPL");
    println!();
    println!("Device commands (require connection):");
    println!("  sync-time <mac>        Synchronize device clock");
    println!("  firmware <mac>         Read firmware version");
    println!("  battery <mac>          Read battery level");
    println!("  settings-read <mac>    Read device settings");
    println!("  settings-write <mac> [options]");
    println!("  alarm-list <mac>       List alarms");
    println!("  alarm-set <mac> <slot> <HH:MM> [options]");
    println!("  alarm-delete <mac> <slot>");
    println!("  brightness <mac> <0-150>");
    println!("  ringtone-preview <mac> [-v <vol>]");
    println!("  monitor <mac> [-d <secs>]");
}
