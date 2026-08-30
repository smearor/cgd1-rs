use cgd1_rs::Backend;
use cgd1_rs::Brightness;
use cgd1_rs::Language;
use cgd1_rs::MacAddress;
use cgd1_rs::TemperatureUnit;
use cgd1_rs::TimeFormat;
use cgd1_rs::Timezone;
use cgd1_rs::Volume;

use crate::connection::DeviceConnection;
use crate::error::CliError;

/// Arguments for the `settings-write` command.
pub struct SettingsWriteArgs {
    /// Device MAC address.
    pub address: MacAddress,
    /// Volume (1–5).
    pub volume: Option<Volume>,
    /// Brightness (0–150, multiple of 10).
    pub brightness: Option<Brightness>,
    /// Night brightness (0–150, multiple of 10).
    pub night_brightness: Option<Brightness>,
    /// Timezone offset in minutes (-720 to +840).
    pub timezone: Option<Timezone>,
    /// Time format: 12 or 24.
    pub time_format: Option<TimeFormat>,
    /// Temperature unit: C or F.
    pub temp_unit: Option<TemperatureUnit>,
    /// Language: en or zh.
    pub language: Option<Language>,
    /// BLE backend to use.
    pub backend: Backend,
}

/// Run the `settings-write` command.
pub async fn run(args: SettingsWriteArgs) -> Result<(), CliError> {
    let connection = DeviceConnection::connect(&args.address, args.backend).await?;
    let mut settings = connection.device().read_settings().await?;

    if let Some(vol) = args.volume {
        settings = settings.with_volume(vol)?;
    }
    if let Some(b) = args.brightness {
        settings = settings.with_brightness(b)?;
    }
    if let Some(nb) = args.night_brightness {
        settings = settings.with_night_brightness(nb)?;
    }
    if let Some(tz) = args.timezone {
        settings = settings.with_timezone(tz)?;
    }
    if let Some(tf) = args.time_format {
        settings = settings.with_time_format(tf);
    }
    if let Some(tu) = args.temp_unit {
        settings = settings.with_temperature_unit(tu);
    }
    if let Some(lang) = args.language {
        settings = settings.with_language(lang);
    }

    connection.device().write_settings(&settings).await?;
    println!("Settings updated.");
    Ok(())
}
