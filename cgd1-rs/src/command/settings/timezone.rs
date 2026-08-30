use std::fmt;
use std::fmt::Display;
use std::fmt::Formatter;
use std::str::FromStr;

use crate::error::ClockError;
use crate::error::Result;
use serde::Deserialize;
use serde::Serialize;
use thiserror::Error;

/// Error parsing a [`Timezone`] from a string.
#[derive(Debug, Clone, PartialEq, Eq, Error)]
#[error("invalid timezone '{input}': {reason}")]
pub struct TimezoneParseError {
    /// The raw input string.
    pub input: String,
    /// The parse error reason.
    pub reason: String,
}

/// Timezone offset encoded in 6-minute units as used by the CGD1 protocol.
///
/// The device stores timezone as `offset_minutes / 6` with a separate sign byte.
/// This newtype encapsulates the encoding/decoding and validates the range.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
pub struct Timezone {
    /// Timezone offset in minutes (e.g., +60 for UTC+1, -300 for UTC-5).
    offset_minutes: i16,
}

impl Timezone {
    /// Maximum offset in minutes (+840 = UTC+14).
    pub const MAX_MINUTES: i16 = 840;
    /// Minimum offset in minutes (-720 = UTC-12).
    pub const MIN_MINUTES: i16 = -720;

    /// Create a timezone from an offset in minutes. Range: -720 to +840.
    pub fn from_minutes(minutes: i16) -> Result<Self> {
        if !(Self::MIN_MINUTES..=Self::MAX_MINUTES).contains(&minutes) {
            return Err(ClockError::InvalidSettings(format!("timezone {minutes} out of range -720..840")));
        }
        Ok(Self { offset_minutes: minutes })
    }

    /// Create a timezone from an offset in hours. Range: -12 to +14.
    pub fn from_hours(hours: i8) -> Result<Self> {
        Self::from_minutes(hours as i16 * 60)
    }

    /// Get the offset in minutes.
    pub const fn minutes(self) -> i16 {
        self.offset_minutes
    }

    /// Get the offset in hours (truncated, may lose fractional part).
    pub const fn hours(self) -> i8 {
        (self.offset_minutes / 60) as i8
    }

    /// Encode as the 6-minute unit value used by the protocol.
    pub fn encoded_units(self) -> u8 {
        (self.offset_minutes.unsigned_abs() / 6) as u8
    }

    /// Encode the sign byte: `0x01` for positive/zero, `0x00` for negative.
    pub fn sign_byte(self) -> u8 {
        if self.offset_minutes < 0 { 0x00 } else { 0x01 }
    }

    /// Decode from the protocol's 6-minute unit value and sign byte.
    pub fn from_encoded(units: u8, sign: u8) -> Result<Self> {
        let abs_minutes = (units as i16) * 6;
        let minutes = if sign == 0x00 { -abs_minutes } else { abs_minutes };
        Self::from_minutes(minutes)
    }
}

impl Display for Timezone {
    fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
        let hours = self.offset_minutes / 60;
        let mins = self.offset_minutes % 60;
        if mins == 0 {
            write!(f, "{hours:+}")
        } else {
            write!(f, "{hours:+}:{mins:02}")
        }
    }
}

impl FromStr for Timezone {
    type Err = TimezoneParseError;

    fn from_str(s: &str) -> std::result::Result<Self, Self::Err> {
        let trimmed = s.trim_start_matches("UTC").trim_start_matches("utc");
        let parsed: i16 = i16::from_str(trimmed).map_err(|e| TimezoneParseError {
            input: s.to_string(),
            reason: e.to_string(),
        })?;
        Self::from_minutes(parsed).map_err(|e| TimezoneParseError {
            input: s.to_string(),
            reason: e.to_string(),
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn from_hours_utc() {
        let tz = Timezone::from_hours(0).unwrap();
        assert_eq!(tz.minutes(), 0);
        assert_eq!(tz.hours(), 0);
    }

    #[test]
    fn from_hours_positive() {
        let tz = Timezone::from_hours(5).unwrap();
        assert_eq!(tz.minutes(), 300);
        assert_eq!(tz.encoded_units(), 50);
        assert_eq!(tz.sign_byte(), 0x01);
    }

    #[test]
    fn from_hours_negative() {
        let tz = Timezone::from_hours(-8).unwrap();
        assert_eq!(tz.minutes(), -480);
        assert_eq!(tz.encoded_units(), 80);
        assert_eq!(tz.sign_byte(), 0x00);
    }

    #[test]
    fn from_minutes_fractional() {
        // India: UTC+5:30 = 330 minutes
        let tz = Timezone::from_minutes(330).unwrap();
        assert_eq!(tz.minutes(), 330);
        assert_eq!(tz.encoded_units(), 55);
        assert_eq!(tz.sign_byte(), 0x01);
    }

    #[test]
    fn from_encoded_roundtrip() {
        let tz = Timezone::from_hours(3).unwrap();
        let decoded = Timezone::from_encoded(tz.encoded_units(), tz.sign_byte()).unwrap();
        assert_eq!(tz, decoded);
    }

    #[test]
    fn from_encoded_negative_roundtrip() {
        let tz = Timezone::from_hours(-5).unwrap();
        let decoded = Timezone::from_encoded(tz.encoded_units(), tz.sign_byte()).unwrap();
        assert_eq!(tz, decoded);
    }

    #[test]
    fn rejects_out_of_range() {
        assert!(Timezone::from_hours(15).is_err());
        assert!(Timezone::from_hours(-13).is_err());
        assert!(Timezone::from_minutes(900).is_err());
        assert!(Timezone::from_minutes(-800).is_err());
    }

    #[test]
    fn boundary_values() {
        assert!(Timezone::from_hours(14).is_ok());
        assert!(Timezone::from_hours(-12).is_ok());
    }
}
