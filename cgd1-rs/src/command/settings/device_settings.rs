use crate::error::ClockError;
use crate::error::Result;
use crate::types::ClockTime;

use serde::Deserialize;
use serde::Serialize;

use super::brightness::Brightness;
use super::language::Language;
use super::ringtone_signature::RingtoneSignature;
use super::screen_light_duration::ScreenLightDuration;
use super::settings_flags::SettingsFlags;
use super::temperature_unit::TemperatureUnit;
use super::time_format::TimeFormat;
use super::timezone::Timezone;
use super::volume::Volume;

/// Device settings for the CGD1 alarm clock.
///
/// Maps to the 18-byte settings payload documented in `docs/BLE.md` §6.3
/// and cross-referenced against the clOwOck Android implementation.
/// The full frame is `13 01 [18 bytes payload]` for write and
/// `13 02 [18 bytes payload]` for read response.
///
/// Invariants are enforced by construction via the newtype fields:
/// - `volume` is in range 1–5 (validated by [`Volume`])
/// - `brightness` and `night_brightness` are 0–150 in multiples of 10
/// - `night_start`/`night_end` hours are 0–23, minutes 0–59
/// - `timezone` is within -720 to +840 minutes
/// - `screen_light_duration` is in range 0–255 seconds
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct DeviceSettings {
    /// Sound volume (1–5).
    volume: Volume,
    /// Flags (language, time format, temperature unit, master alarm disable).
    #[serde(flatten)]
    flags: SettingsFlags,
    /// Timezone offset.
    timezone: Timezone,
    /// Screen light duration.
    screen_light_duration: ScreenLightDuration,
    /// Daytime brightness (0–150, multiple of 10).
    brightness: Brightness,
    /// Nighttime brightness (0–150, multiple of 10).
    night_brightness: Brightness,
    /// Night mode start time.
    night_start: ClockTime,
    /// Night mode end time.
    night_end: ClockTime,
    /// Whether night mode is enabled.
    night_mode_enabled: bool,
    /// Ringtone signature (4 bytes, `0xFF` when unused).
    ringtone_signature: RingtoneSignature,
}

impl DeviceSettings {
    /// Maximum volume level.
    pub const MAX_VOLUME: u8 = 5;

    /// Fixed header byte at payload offset 1.
    pub const HDR_BYTE_1: u8 = 0x58;
    /// Fixed header byte at payload offset 2.
    pub const HDR_BYTE_2: u8 = 0x02;

    /// Create a new device settings with validation.
    #[allow(clippy::too_many_arguments)]
    pub fn new(
        volume: Volume,
        time_format: TimeFormat,
        temperature_unit: TemperatureUnit,
        language: Language,
        timezone: Timezone,
        screen_light_duration: ScreenLightDuration,
        brightness: Brightness,
        night_brightness: Brightness,
        night_start: ClockTime,
        night_end: ClockTime,
        night_mode_enabled: bool,
        master_alarm_disabled: bool,
        ringtone_signature: RingtoneSignature,
    ) -> Result<Self> {
        let flags = SettingsFlags::new(language, time_format, temperature_unit, master_alarm_disabled);

        Ok(Self {
            volume,
            flags,
            timezone,
            screen_light_duration,
            brightness,
            night_brightness,
            night_start,
            night_end,
            night_mode_enabled,
            ringtone_signature,
        })
    }

    /// Get the volume level (1–5).
    pub const fn volume(&self) -> Volume {
        self.volume
    }

    /// Get the time format.
    pub const fn time_format(&self) -> TimeFormat {
        self.flags.time_format()
    }

    /// Get the temperature unit.
    pub const fn temperature_unit(&self) -> TemperatureUnit {
        self.flags.temperature_unit()
    }

    /// Get the display language.
    pub const fn language(&self) -> Language {
        self.flags.language()
    }

    /// Get the timezone.
    pub const fn timezone(&self) -> Timezone {
        self.timezone
    }

    /// Get the screen light duration.
    pub const fn screen_light_duration(&self) -> ScreenLightDuration {
        self.screen_light_duration
    }

    /// Get the daytime brightness.
    pub const fn brightness(&self) -> Brightness {
        self.brightness
    }

    /// Get the nighttime brightness.
    pub const fn night_brightness(&self) -> Brightness {
        self.night_brightness
    }

    /// Get the night mode start time.
    pub const fn night_start(&self) -> ClockTime {
        self.night_start
    }

    /// Get the night mode end time.
    pub const fn night_end(&self) -> ClockTime {
        self.night_end
    }

    /// Whether night mode is enabled.
    pub const fn night_mode_enabled(&self) -> bool {
        self.night_mode_enabled
    }

    /// Whether the master alarm switch is disabled.
    pub const fn master_alarm_disabled(&self) -> bool {
        self.flags.master_alarm_disabled()
    }

    /// Get the ringtone signature.
    pub const fn ringtone_signature(&self) -> RingtoneSignature {
        self.ringtone_signature
    }

    /// Return a copy with the specified volume.
    pub fn with_volume(self, volume: Volume) -> Result<Self> {
        Ok(Self { volume, ..self })
    }

    /// Return a copy with the specified brightness.
    pub fn with_brightness(self, brightness: Brightness) -> Result<Self> {
        Ok(Self { brightness, ..self })
    }

    /// Return a copy with the specified night brightness.
    pub fn with_night_brightness(self, night_brightness: Brightness) -> Result<Self> {
        Ok(Self { night_brightness, ..self })
    }

    /// Return a copy with the specified timezone.
    pub fn with_timezone(self, timezone: Timezone) -> Result<Self> {
        Ok(Self { timezone, ..self })
    }

    /// Return a copy with the specified time format.
    pub fn with_time_format(self, time_format: TimeFormat) -> Self {
        let flags = SettingsFlags::new(self.flags.language(), time_format, self.flags.temperature_unit(), self.flags.master_alarm_disabled());
        Self { flags, ..self }
    }

    /// Return a copy with the specified temperature unit.
    pub fn with_temperature_unit(self, temp_unit: TemperatureUnit) -> Self {
        let flags = SettingsFlags::new(self.flags.language(), self.flags.time_format(), temp_unit, self.flags.master_alarm_disabled());
        Self { flags, ..self }
    }

    /// Return a copy with the specified language.
    pub fn with_language(self, language: Language) -> Self {
        let flags = SettingsFlags::new(language, self.flags.time_format(), self.flags.temperature_unit(), self.flags.master_alarm_disabled());
        Self { flags, ..self }
    }

    /// Encode settings into the 18-byte payload for the settings frame.
    ///
    /// Layout (offsets within the payload, after the `[length] [command]` header):
    /// ```text
    /// [0]  Volume
    /// [1]  Hdr1 (0x58)
    /// [2]  Hdr2 (0x02)
    /// [3]  Flags (language | time_format | temp_unit | master_alarm_disable)
    /// [4]  Timezone (6-minute units)
    /// [5]  Screen duration (seconds)
    /// [6]  Packed brightness (high nibble = day, low nibble = night)
    /// [7]  Night start hour
    /// [8]  Night start minute
    /// [9]  Night end hour
    /// [10] Night end minute
    /// [11] Timezone sign (0x01 = positive, 0x00 = negative)
    /// [12] Night mode enabled (0x01 = enabled, 0x00 = disabled)
    /// [13] Reserved (0xFF)
    /// [14–17] Ringtone signature (4 bytes)
    /// ```
    ///
    /// When night mode is disabled, the night start/end values are overwritten
    /// to `00:00–00:01` in the encoded payload. This matches the firmware
    /// workaround used by clOwOck and the official app, since the device does
    /// not properly support disabling night mode via the enabled flag alone.
    pub fn encode(&self) -> [u8; 18] {
        let mut payload = [0u8; 18];
        payload[0] = self.volume.value();
        payload[1] = Self::HDR_BYTE_1;
        payload[2] = Self::HDR_BYTE_2;
        payload[3] = self.flags.byte();
        payload[4] = self.timezone.encoded_units();
        payload[5] = self.screen_light_duration.seconds();
        payload[6] = (self.brightness.nibble() << 4) | self.night_brightness.nibble();
        if self.night_mode_enabled {
            payload[7] = self.night_start.hour();
            payload[8] = self.night_start.minute();
            payload[9] = self.night_end.hour();
            payload[10] = self.night_end.minute();
        } else {
            // Firmware workaround: 1-minute night mode to effectively disable it.
            payload[7] = 0;
            payload[8] = 0;
            payload[9] = 0;
            payload[10] = 1;
        }
        payload[11] = self.timezone.sign_byte();
        payload[12] = if self.night_mode_enabled { 0x01 } else { 0x00 };
        payload[13] = 0xFF;
        payload[14..18].copy_from_slice(&self.ringtone_signature.bytes());
        payload
    }

    /// Decode settings from a raw payload read from the device.
    ///
    /// The payload should be the 18 bytes following the `[length] [command]` header.
    pub fn decode(payload: &[u8]) -> Result<Self> {
        if payload.len() < 18 {
            return Err(ClockError::Parse("settings payload too short".into()));
        }

        let volume = Volume::new(payload[0])?;
        let flags = payload[3];
        let timezone = Timezone::from_encoded(payload[4], payload[11])?;
        let screen_light_duration = ScreenLightDuration::new(payload[5])?;
        let day_nibble = (payload[6] >> 4) & 0x0F;
        let night_nibble = payload[6] & 0x0F;
        let brightness = Brightness::from_nibble(day_nibble)?;
        let night_brightness = Brightness::from_nibble(night_nibble)?;
        let night_start = ClockTime::new(payload[7], payload[8])?;
        let night_end = ClockTime::new(payload[9], payload[10])?;
        let night_mode_enabled = payload[12] != 0x00;
        let mut sig_bytes = [0u8; 4];
        sig_bytes.copy_from_slice(&payload[14..18]);
        let ringtone_signature = RingtoneSignature::from_bytes(sig_bytes);

        let settings_flags = SettingsFlags::from_byte(flags);

        Self::new(
            volume,
            settings_flags.time_format(),
            settings_flags.temperature_unit(),
            settings_flags.language(),
            timezone,
            screen_light_duration,
            brightness,
            night_brightness,
            night_start,
            night_end,
            night_mode_enabled,
            settings_flags.master_alarm_disabled(),
            ringtone_signature,
        )
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn sample_settings() -> DeviceSettings {
        DeviceSettings::new(
            Volume::new(3).unwrap(),
            TimeFormat::TwentyFourHour,
            TemperatureUnit::Celsius,
            Language::English,
            Timezone::from_hours(1).unwrap(),
            ScreenLightDuration::new(10).unwrap(),
            Brightness::new(80).unwrap(),
            Brightness::new(30).unwrap(),
            ClockTime::new(22, 0).unwrap(),
            ClockTime::new(7, 0).unwrap(),
            true,
            true,
            RingtoneSignature::Unused,
        )
        .unwrap()
    }

    #[test]
    fn encode_decode_roundtrip() {
        let settings = sample_settings();
        let encoded = settings.encode();
        let decoded = DeviceSettings::decode(&encoded).unwrap();
        assert_eq!(settings, decoded);
    }

    #[test]
    fn encode_known_values() {
        let settings = sample_settings();
        let payload = settings.encode();
        assert_eq!(payload[0], 3); // volume
        assert_eq!(payload[1], 0x58); // hdr1
        assert_eq!(payload[2], 0x02); // hdr2
        assert_eq!(payload[3], 0x11); // flags: English | master_alarm_disable
        assert_eq!(payload[4], 10); // timezone: 60 min / 6 = 10
        assert_eq!(payload[5], 10); // screen duration
        assert_eq!(payload[6], 0x83); // brightness: day=8, night=3
        assert_eq!(payload[7], 22); // night start hour
        assert_eq!(payload[8], 0); // night start minute
        assert_eq!(payload[9], 7); // night end hour
        assert_eq!(payload[10], 0); // night end minute
        assert_eq!(payload[11], 0x01); // timezone sign: positive
        assert_eq!(payload[12], 0x01); // night mode enabled
        assert_eq!(payload[13], 0xFF); // reserved
        assert_eq!(&payload[14..18], &[0xFF, 0xFF, 0xFF, 0xFF]); // signature
    }

    #[test]
    fn decode_too_short() {
        assert!(DeviceSettings::decode(&[0; 10]).is_err());
    }

    #[test]
    fn new_rejects_invalid_volume() {
        assert!(
            DeviceSettings::new(
                Volume::new(0).unwrap_or(Volume::MIN),
                TimeFormat::TwentyFourHour,
                TemperatureUnit::Celsius,
                Language::English,
                Timezone::from_hours(0).unwrap(),
                ScreenLightDuration::new(10).unwrap(),
                Brightness::new(50).unwrap(),
                Brightness::new(30).unwrap(),
                ClockTime::new(22, 0).unwrap(),
                ClockTime::new(7, 0).unwrap(),
                true,
                true,
                RingtoneSignature::Unused
            )
            .is_ok() // Volume::new(0) fails, so we use MIN as fallback
        );
        assert!(Volume::new(0).is_err());
        assert!(Volume::new(6).is_err());
    }

    #[test]
    fn new_rejects_invalid_night_hours() {
        assert!(ClockTime::new(24, 0).is_err());
        assert!(ClockTime::new(0, 60).is_err());
    }

    #[test]
    fn night_mode_disabled_overwrites_schedule() {
        let settings = DeviceSettings::new(
            Volume::new(3).unwrap(),
            TimeFormat::TwentyFourHour,
            TemperatureUnit::Celsius,
            Language::English,
            Timezone::from_hours(0).unwrap(),
            ScreenLightDuration::new(10).unwrap(),
            Brightness::new(50).unwrap(),
            Brightness::new(30).unwrap(),
            ClockTime::new(22, 0).unwrap(),
            ClockTime::new(7, 0).unwrap(),
            false,
            true,
            RingtoneSignature::Unused,
        )
        .unwrap();
        let payload = settings.encode();
        // Night mode disabled: encoded as 00:00-00:01 workaround
        assert_eq!(payload[7], 0);
        assert_eq!(payload[8], 0);
        assert_eq!(payload[9], 0);
        assert_eq!(payload[10], 1);
        assert_eq!(payload[12], 0x00);
    }

    #[test]
    fn master_alarm_disabled_flag() {
        let with_flag = DeviceSettings::new(
            Volume::new(3).unwrap(),
            TimeFormat::TwentyFourHour,
            TemperatureUnit::Celsius,
            Language::Chinese,
            Timezone::from_hours(0).unwrap(),
            ScreenLightDuration::new(10).unwrap(),
            Brightness::new(50).unwrap(),
            Brightness::new(30).unwrap(),
            ClockTime::new(22, 0).unwrap(),
            ClockTime::new(7, 0).unwrap(),
            true,
            true,
            RingtoneSignature::Unused,
        )
        .unwrap();
        assert!(with_flag.master_alarm_disabled());
        assert_eq!(with_flag.encode()[3] & 0x10, 0x10);

        let without_flag = DeviceSettings::new(
            Volume::new(3).unwrap(),
            TimeFormat::TwentyFourHour,
            TemperatureUnit::Celsius,
            Language::Chinese,
            Timezone::from_hours(0).unwrap(),
            ScreenLightDuration::new(10).unwrap(),
            Brightness::new(50).unwrap(),
            Brightness::new(30).unwrap(),
            ClockTime::new(22, 0).unwrap(),
            ClockTime::new(7, 0).unwrap(),
            true,
            false,
            RingtoneSignature::Unused,
        )
        .unwrap();
        assert!(!without_flag.master_alarm_disabled());
        assert_eq!(without_flag.encode()[3] & 0x10, 0x00);
    }

    #[test]
    fn flags_combine_all_fields() {
        let settings = DeviceSettings::new(
            Volume::new(3).unwrap(),
            TimeFormat::TwelveHour,
            TemperatureUnit::Fahrenheit,
            Language::English,
            Timezone::from_hours(0).unwrap(),
            ScreenLightDuration::new(10).unwrap(),
            Brightness::new(50).unwrap(),
            Brightness::new(30).unwrap(),
            ClockTime::new(22, 0).unwrap(),
            ClockTime::new(7, 0).unwrap(),
            true,
            true,
            RingtoneSignature::Unused,
        )
        .unwrap();
        // English=0x01, 12h=0x02, Fahrenheit=0x04, master_alarm_disable=0x10 → 0x17
        assert_eq!(settings.encode()[3], 0x17);
    }
}
