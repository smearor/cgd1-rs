use crate::command::AlarmSlotIndexParseError;
use crate::command::BrightnessParseError;
use crate::command::DayMaskParseError;
use crate::command::LanguageParseError;
use crate::command::RingtoneSignatureParseError;
use crate::command::ScreenLightDurationParseError;
use crate::command::TemperatureUnitParseError;
use crate::command::TimeFormatParseError;
use crate::command::TimezoneParseError;
use crate::command::VolumeParseError;
use crate::types::BatteryLevelParseError;
use crate::types::ClockTimeParseError;
use crate::types::HumidityParseError;
use crate::types::MacAddressParseError;
use crate::types::TemperatureParseError;
use thiserror::Error;

/// Library-level result alias.
pub type Result<T> = std::result::Result<T, ClockError>;

/// Authentication failure details.
///
/// Carries the original device-side reason and optional context about
/// whether the token was newly generated or loaded from storage.
#[derive(Debug, Clone, PartialEq, Eq, Error)]
#[error("{reason}")]
pub struct AuthFailedError {
    /// The original reason from the device (e.g. status code).
    pub reason: String,
    /// Whether the token was newly generated (not found in storage).
    pub is_new_token: bool,
    /// Filesystem path to the token file, if applicable.
    pub token_path: Option<String>,
}

/// Errors returned by the cgd1-rs library.
#[derive(Debug, thiserror::Error)]
pub enum ClockError {
    /// BLE transport error.
    #[error("BLE transport error: {0}")]
    Transport(String),

    /// Authentication failed (token rejected by device).
    #[error("{0}")]
    AuthFailed(AuthFailedError),

    /// No authentication token available for this device.
    #[error("no auth token: device not paired")]
    NoAuthToken,

    /// Command was rejected by the device (non-success ACK status).
    #[error("command rejected: command={command:#04x}, status={status}")]
    CommandRejected {
        /// The command byte that was rejected.
        command: u8,
        /// The ACK status from the device.
        status: crate::command::AckStatus,
    },

    /// Timeout waiting for a response from the device.
    #[error("timeout waiting for response")]
    Timeout,

    /// Device is not connected.
    #[error("not connected")]
    NotConnected,

    /// Device is already connected.
    #[error("already connected")]
    AlreadyConnected,

    /// Invalid alarm slot index (must be 0-15).
    #[error("invalid alarm slot: {0}")]
    InvalidAlarmSlot(u8),

    /// Invalid settings value (out of range).
    #[error("invalid settings value: {0}")]
    InvalidSettings(String),

    /// Failed to parse an advertisement or notification.
    #[error("parse error: {0}")]
    Parse(String),

    /// Failed to parse a day mask from a string.
    #[error(transparent)]
    DayMaskParse(#[from] DayMaskParseError),

    /// Failed to parse an alarm slot index from a string.
    #[error(transparent)]
    AlarmSlotIndexParse(#[from] AlarmSlotIndexParseError),

    /// Failed to parse a MAC address from a string.
    #[error(transparent)]
    MacAddressParse(#[from] MacAddressParseError),

    /// Failed to parse a battery level from a string.
    #[error(transparent)]
    BatteryLevelParse(#[from] BatteryLevelParseError),

    /// Failed to parse a humidity from a string.
    #[error(transparent)]
    HumidityParse(#[from] HumidityParseError),

    /// Failed to parse a temperature from a string.
    #[error(transparent)]
    TemperatureParse(#[from] TemperatureParseError),

    /// Failed to parse a brightness from a string.
    #[error(transparent)]
    BrightnessParse(#[from] BrightnessParseError),

    /// Failed to parse a language from a string.
    #[error(transparent)]
    LanguageParse(#[from] LanguageParseError),

    /// Failed to parse a time format from a string.
    #[error(transparent)]
    TimeFormatParse(#[from] TimeFormatParseError),

    /// Failed to parse a timezone from a string.
    #[error(transparent)]
    TimezoneParse(#[from] TimezoneParseError),

    /// Failed to parse a temperature unit from a string.
    #[error(transparent)]
    TemperatureUnitParse(#[from] TemperatureUnitParseError),

    /// Failed to parse a volume from a string.
    #[error(transparent)]
    VolumeParse(#[from] VolumeParseError),

    /// Failed to parse a clock time from a string.
    #[error(transparent)]
    ClockTimeParse(#[from] ClockTimeParseError),

    /// Failed to parse a screen duration from a string.
    #[error(transparent)]
    ScreenLightDurationParse(#[from] ScreenLightDurationParseError),

    /// Failed to parse a ringtone signature from a string.
    #[error(transparent)]
    RingtoneSignatureParse(#[from] RingtoneSignatureParseError),

    /// I/O error.
    #[error("IO error: {0}")]
    Io(#[from] std::io::Error),

    /// Internal btleplug error.
    #[error("btleplug error: {0}")]
    Btleplug(#[from] btleplug::Error),
}
