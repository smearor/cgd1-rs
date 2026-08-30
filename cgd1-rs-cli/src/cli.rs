use std::path::PathBuf;

use cgd1_rs::AlarmSlotIndex;
use cgd1_rs::Backend;
use cgd1_rs::Brightness;
use cgd1_rs::ClockTime;
use cgd1_rs::DayMask;
use cgd1_rs::Language;
use cgd1_rs::MacAddress;
use cgd1_rs::RingtoneSignature;
use cgd1_rs::ScanDuration;
use cgd1_rs::TemperatureUnit;
use cgd1_rs::TimeFormat;
use cgd1_rs::Timezone;
use cgd1_rs::Volume;
use clap::Parser;
use clap::Subcommand;

use crate::monitor_duration::MonitorDuration;

/// Command-line tool for the Qingping CGD1 Bluetooth Alarm Clock.
#[derive(Parser)]
#[command(name = "cgd1", version, about = "Control Qingping CGD1 via BLE")]
pub struct Cli {
    /// Verbosity level (-v, -vv, -vvv)
    #[arg(short, long, action = clap::ArgAction::Count)]
    pub verbose: u8,

    /// BLE backend: `btleplug` (real hardware) or `virtual` (in-memory device for testing).
    #[arg(long, default_value_t)]
    pub backend: Backend,

    #[command(subcommand)]
    pub command: Commands,
}

#[derive(Subcommand)]
pub enum Commands {
    /// Scan for nearby CGD1 devices.
    Scan {
        /// Scan duration in seconds (1–600).
        #[arg(short, long, default_value = "10")]
        duration: ScanDuration,
    },

    /// Synchronize the device clock to the current system time.
    SyncTime {
        /// Device MAC address.
        address: MacAddress,
    },

    /// Read all alarms from the device.
    AlarmList {
        /// Device MAC address.
        address: MacAddress,
    },

    /// Set an alarm at a specific slot.
    AlarmSet {
        /// Device MAC address.
        address: MacAddress,
        /// Slot index (0–15).
        slot: AlarmSlotIndex,
        /// Alarm time in HH:MM format (e.g., "07:30").
        time: ClockTime,
        /// Repeat mask as hex (e.g., 7f for every day, 3e for weekdays).
        #[arg(short, long, default_value = "7f")]
        repeat: DayMask,
        /// Disable snooze for this alarm.
        #[arg(long)]
        no_snooze: bool,
    },

    /// Delete an alarm at a specific slot.
    AlarmDelete {
        /// Device MAC address.
        address: MacAddress,
        /// Slot index (0–15).
        slot: AlarmSlotIndex,
    },

    /// Read device settings.
    SettingsRead {
        /// Device MAC address.
        address: MacAddress,
    },

    /// Write device settings.
    SettingsWrite {
        /// Device MAC address.
        address: MacAddress,
        /// Volume (1–5).
        #[arg(long)]
        volume: Option<Volume>,
        /// Brightness (0–150, multiple of 10).
        #[arg(long)]
        brightness: Option<Brightness>,
        /// Night brightness (0–150, multiple of 10).
        #[arg(long)]
        night_brightness: Option<Brightness>,
        /// Timezone offset in minutes (-720 to +840).
        #[arg(long)]
        timezone: Option<Timezone>,
        /// Time format: 12 or 24.
        #[arg(long)]
        time_format: Option<TimeFormat>,
        /// Temperature unit: C or F.
        #[arg(long)]
        temp_unit: Option<TemperatureUnit>,
        /// Language: en or zh.
        #[arg(long)]
        language: Option<Language>,
    },

    /// Set immediate brightness (preview).
    Brightness {
        /// Device MAC address.
        address: MacAddress,
        /// Brightness value (0–150, multiple of 10).
        value: Brightness,
    },

    /// Preview a ringtone.
    RingtonePreview {
        /// Device MAC address.
        address: MacAddress,
        /// Volume level (optional, uses device volume if omitted).
        #[arg(short, long)]
        volume: Option<Volume>,
    },

    /// Upload a custom ringtone from a file.
    RingtoneUpload {
        /// Device MAC address.
        address: MacAddress,
        /// Path to 8-bit PCM audio file (8 kHz, mono).
        file: PathBuf,
        /// Ringtone name (e.g., "Beep", "CustomSlotA") or 4-byte hex (e.g., "deadbeef").
        #[arg(short, long, default_value = "CustomSlotA")]
        signature: RingtoneSignature,
    },

    /// Read firmware version.
    Firmware {
        /// Device MAC address.
        address: MacAddress,
    },

    /// Read battery level.
    Battery {
        /// Device MAC address.
        address: MacAddress,
    },

    /// Monitor sensor data (temperature, humidity) in real-time.
    Monitor {
        /// Device MAC address.
        address: MacAddress,
        /// Duration in seconds (0 = indefinite).
        #[arg(short, long, default_value = "0")]
        duration: MonitorDuration,
    },

    /// Start an interactive REPL session.
    ///
    /// Keeps a persistent connection to the device so that state changes
    /// (e.g. `settings-write`) are visible in subsequent commands
    /// (e.g. `settings-read`). If `--address` is given, the REPL connects
    /// automatically on startup; otherwise use `connect <mac>` inside.
    Repl {
        /// Device MAC address (optional; use `connect` inside the REPL if omitted).
        #[arg(long)]
        address: Option<MacAddress>,
    },
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::str::FromStr;

    #[test]
    fn parse_scan_default_duration() {
        let cli = Cli::try_parse_from(["cgd1", "scan"]).unwrap();
        match cli.command {
            Commands::Scan { duration } => assert_eq!(duration.seconds(), 10),
            _ => panic!("expected Scan command"),
        }
    }

    #[test]
    fn parse_scan_custom_duration() {
        let cli = Cli::try_parse_from(["cgd1", "scan", "-d", "30"]).unwrap();
        match cli.command {
            Commands::Scan { duration } => assert_eq!(duration.seconds(), 30),
            _ => panic!("expected Scan command"),
        }
    }

    #[test]
    fn parse_sync_time() {
        let cli = Cli::try_parse_from(["cgd1", "sync-time", "aa:bb:cc:dd:ee:ff"]).unwrap();
        match cli.command {
            Commands::SyncTime { address } => assert_eq!(address.to_string(), "aa:bb:cc:dd:ee:ff"),
            _ => panic!("expected SyncTime command"),
        }
    }

    #[test]
    fn parse_alarm_set_with_defaults() {
        let cli = Cli::try_parse_from(["cgd1", "alarm-set", "aa:bb:cc:dd:ee:ff", "3", "07:30"]).unwrap();
        match cli.command {
            Commands::AlarmSet {
                slot, time, repeat, no_snooze, ..
            } => {
                assert_eq!(slot.value(), 3);
                assert_eq!(time.hour(), 7);
                assert_eq!(time.minute(), 30);
                assert_eq!(repeat, DayMask::EVERY_DAY);
                assert!(!no_snooze);
            }
            _ => panic!("expected AlarmSet command"),
        }
    }

    #[test]
    fn parse_alarm_set_with_options() {
        let cli = Cli::try_parse_from(["cgd1", "alarm-set", "aa:bb:cc:dd:ee:ff", "0", "06:00", "-r", "3e", "--no-snooze"]).unwrap();
        match cli.command {
            Commands::AlarmSet { repeat, no_snooze, .. } => {
                assert_eq!(repeat, DayMask::WEEKDAYS);
                assert!(no_snooze);
            }
            _ => panic!("expected AlarmSet command"),
        }
    }

    #[test]
    fn parse_alarm_delete() {
        let cli = Cli::try_parse_from(["cgd1", "alarm-delete", "aa:bb:cc:dd:ee:ff", "5"]).unwrap();
        match cli.command {
            Commands::AlarmDelete { slot, .. } => assert_eq!(slot.value(), 5),
            _ => panic!("expected AlarmDelete command"),
        }
    }

    #[test]
    fn parse_settings_write_partial() {
        let cli = Cli::try_parse_from(["cgd1", "settings-write", "aa:bb:cc:dd:ee:ff", "--volume", "3", "--timezone", "2"]).unwrap();
        match cli.command {
            Commands::SettingsWrite {
                volume, timezone, brightness, ..
            } => {
                assert_eq!(volume.unwrap().value(), 3);
                assert_eq!(timezone.unwrap().minutes(), 2);
                assert_eq!(brightness, None);
            }
            _ => panic!("expected SettingsWrite command"),
        }
    }

    #[test]
    fn parse_brightness() {
        let cli = Cli::try_parse_from(["cgd1", "brightness", "aa:bb:cc:dd:ee:ff", "80"]).unwrap();
        match cli.command {
            Commands::Brightness { value, .. } => assert_eq!(value.value(), 80),
            _ => panic!("expected Brightness command"),
        }
    }

    #[test]
    fn parse_ringtone_preview_with_volume() {
        let cli = Cli::try_parse_from(["cgd1", "ringtone-preview", "aa:bb:cc:dd:ee:ff", "-v", "4"]).unwrap();
        match cli.command {
            Commands::RingtonePreview { volume, .. } => assert_eq!(volume.unwrap().value(), 4),
            _ => panic!("expected RingtonePreview command"),
        }
    }

    #[test]
    fn parse_ringtone_upload_default_signature() {
        let cli = Cli::try_parse_from(["cgd1", "ringtone-upload", "aa:bb:cc:dd:ee:ff", "audio.pcm"]).unwrap();
        match cli.command {
            Commands::RingtoneUpload { file, signature, .. } => {
                assert_eq!(file, PathBuf::from("audio.pcm"));
                assert_eq!(signature, RingtoneSignature::CustomSlotA);
            }
            _ => panic!("expected RingtoneUpload command"),
        }
    }

    #[test]
    fn parse_ringtone_upload_custom_signature() {
        let cli = Cli::try_parse_from(["cgd1", "ringtone-upload", "aa:bb:cc:dd:ee:ff", "audio.pcm", "-s", "beefbeef"]).unwrap();
        match cli.command {
            Commands::RingtoneUpload { signature, .. } => assert_eq!(signature, RingtoneSignature::from_str("beefbeef").unwrap()),
            _ => panic!("expected RingtoneUpload command"),
        }
    }

    #[test]
    fn parse_monitor_indefinite() {
        let cli = Cli::try_parse_from(["cgd1", "monitor", "aa:bb:cc:dd:ee:ff"]).unwrap();
        match cli.command {
            Commands::Monitor { duration, .. } => assert!(duration.is_indefinite()),
            _ => panic!("expected Monitor command"),
        }
    }

    #[test]
    fn parse_monitor_timed() {
        let cli = Cli::try_parse_from(["cgd1", "monitor", "aa:bb:cc:dd:ee:ff", "-d", "60"]).unwrap();
        match cli.command {
            Commands::Monitor { duration, .. } => assert_eq!(duration.seconds(), 60),
            _ => panic!("expected Monitor command"),
        }
    }

    #[test]
    fn parse_verbose_flags() {
        let cli = Cli::try_parse_from(["cgd1", "-vvv", "scan"]).unwrap();
        assert_eq!(cli.verbose, 3);
    }

    #[test]
    fn parse_firmware() {
        let cli = Cli::try_parse_from(["cgd1", "firmware", "aa:bb:cc:dd:ee:ff"]).unwrap();
        assert!(matches!(cli.command, Commands::Firmware { .. }));
    }

    #[test]
    fn parse_battery() {
        let cli = Cli::try_parse_from(["cgd1", "battery", "aa:bb:cc:dd:ee:ff"]).unwrap();
        assert!(matches!(cli.command, Commands::Battery { .. }));
    }

    #[test]
    fn parse_alarm_list() {
        let cli = Cli::try_parse_from(["cgd1", "alarm-list", "aa:bb:cc:dd:ee:ff"]).unwrap();
        assert!(matches!(cli.command, Commands::AlarmList { .. }));
    }

    #[test]
    fn parse_settings_read() {
        let cli = Cli::try_parse_from(["cgd1", "settings-read", "aa:bb:cc:dd:ee:ff"]).unwrap();
        assert!(matches!(cli.command, Commands::SettingsRead { .. }));
    }

    #[test]
    fn missing_subcommand_errors() {
        assert!(Cli::try_parse_from(["cgd1"]).is_err());
    }

    #[test]
    fn invalid_subcommand_errors() {
        assert!(Cli::try_parse_from(["cgd1", "frobnicate"]).is_err());
    }
}
