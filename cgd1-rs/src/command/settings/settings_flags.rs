use crate::command::settings::language::Language;
use crate::command::settings::temperature_unit::TemperatureUnit;
use crate::command::settings::time_format::TimeFormat;

use serde::Deserialize;
use serde::Serialize;

/// Packed flags for device settings, decoded into logical fields.
///
/// Encodes four values into a single byte on the wire:
/// - Bit 0: `Language` (0 = Chinese, 1 = English)
/// - Bit 1: `TimeFormat` (0 = 24h, 1 = 12h)
/// - Bit 2: `TemperatureUnit` (0 = Celsius, 1 = Fahrenheit)
/// - Bit 4: `master_alarm_disabled` (0 = enabled, 1 = disabled)
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub struct SettingsFlags {
    /// Display language.
    language: Language,
    /// Time display format.
    time_format: TimeFormat,
    /// Temperature display unit.
    temperature_unit: TemperatureUnit,
    /// Whether the master alarm switch is disabled.
    master_alarm_disabled: bool,
}

impl SettingsFlags {
    /// Flag bit for master alarm disable.
    pub const FLAG_MASTER_ALARM_DISABLE: u8 = 0x10;

    /// Create flags from the individual logical fields.
    pub const fn new(language: Language, time_format: TimeFormat, temperature_unit: TemperatureUnit, master_alarm_disabled: bool) -> Self {
        Self {
            language,
            time_format,
            temperature_unit,
            master_alarm_disabled,
        }
    }

    /// Decode flags from a raw byte.
    pub fn from_byte(byte: u8) -> Self {
        Self {
            language: Language::from_flags(byte),
            time_format: TimeFormat::from_flags(byte),
            temperature_unit: TemperatureUnit::from_flags(byte),
            master_alarm_disabled: (byte & Self::FLAG_MASTER_ALARM_DISABLE) != 0,
        }
    }

    /// Encode flags into the raw byte for the BLE payload.
    pub fn byte(self) -> u8 {
        self.language.flag_bit()
            | self.time_format.flag_bit()
            | self.temperature_unit.flag_bit()
            | if self.master_alarm_disabled { Self::FLAG_MASTER_ALARM_DISABLE } else { 0 }
    }

    /// Get the language.
    pub const fn language(self) -> Language {
        self.language
    }

    /// Get the time format.
    pub const fn time_format(self) -> TimeFormat {
        self.time_format
    }

    /// Get the temperature unit.
    pub const fn temperature_unit(self) -> TemperatureUnit {
        self.temperature_unit
    }

    /// Whether the master alarm switch is disabled.
    pub const fn master_alarm_disabled(self) -> bool {
        self.master_alarm_disabled
    }
}
