use cgd1_rs::Language;
use cgd1_rs::MacAddress;
use cgd1_rs::TemperatureUnit;
use cgd1_rs::TimeFormat;

use crate::connection::DeviceConnection;
use crate::error::CliError;

/// Arguments for the `settings-read` command.
pub struct SettingsReadArgs {
    /// Device MAC address.
    pub address: MacAddress,
}

/// Run the `settings-read` command.
pub async fn run(args: SettingsReadArgs) -> Result<(), CliError> {
    let connection = DeviceConnection::connect(&args.address).await?;
    let settings = connection.device().read_settings().await?;

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

    Ok(())
}
