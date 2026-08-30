use std::fmt;
use std::fmt::Display;
use std::fmt::Formatter;
use std::str::FromStr;

use crate::error::ClockError;
use crate::error::Result;
use crate::types::parse_iso_duration_to_seconds;
use serde::Deserialize;
use serde::Serialize;
use thiserror::Error;

/// Error parsing a [`ScreenLightDuration`] from a string.
#[derive(Debug, Clone, PartialEq, Eq, Error)]
#[error("invalid screen duration '{input}': {reason}")]
pub struct ScreenLightDurationParseError {
    /// The raw input string.
    pub input: String,
    /// The parse error reason.
    pub reason: String,
}

/// Screen light duration in seconds (0–255).
///
/// Encoded as byte 5 of the settings payload.
/// A value of 0 means the screen light stays off after triggering.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
#[serde(transparent)]
pub struct ScreenLightDuration(u8);

impl ScreenLightDuration {
    /// Minimum screen duration in seconds.
    pub const MIN: ScreenLightDuration = ScreenLightDuration(0);
    /// Maximum screen duration in seconds.
    pub const MAX: ScreenLightDuration = ScreenLightDuration(255);

    /// Create a validated screen duration. Must be 0–255 seconds.
    pub fn new(seconds: u8) -> Result<Self> {
        Ok(Self(seconds))
    }

    /// Get the duration in seconds.
    pub const fn seconds(self) -> u8 {
        self.0
    }

    /// Get the raw byte value.
    pub const fn value(self) -> u8 {
        self.0
    }
}

impl TryFrom<u8> for ScreenLightDuration {
    type Error = ClockError;

    fn try_from(value: u8) -> Result<Self> {
        Self::new(value)
    }
}

impl From<ScreenLightDuration> for u8 {
    fn from(d: ScreenLightDuration) -> Self {
        d.0
    }
}

impl Display for ScreenLightDuration {
    fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
        write!(f, "{}s", self.0)
    }
}

impl FromStr for ScreenLightDuration {
    type Err = ScreenLightDurationParseError;

    fn from_str(s: &str) -> std::result::Result<Self, Self::Err> {
        let seconds: u8 = if s.starts_with('P') {
            let secs = parse_iso_duration_to_seconds(s).ok_or_else(|| ScreenLightDurationParseError {
                input: s.to_string(),
                reason: "invalid ISO 8601 duration".to_string(),
            })?;
            u8::try_from(secs).map_err(|_| ScreenLightDurationParseError {
                input: s.to_string(),
                reason: format!("{secs} seconds exceeds u8 range (0-255)"),
            })?
        } else {
            u8::from_str(s).map_err(|e| ScreenLightDurationParseError {
                input: s.to_string(),
                reason: e.to_string(),
            })?
        };
        Self::new(seconds).map_err(|e| ScreenLightDurationParseError {
            input: s.to_string(),
            reason: e.to_string(),
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn new_valid() {
        assert_eq!(ScreenLightDuration::new(0).unwrap().seconds(), 0);
        assert_eq!(ScreenLightDuration::new(10).unwrap().seconds(), 10);
        assert_eq!(ScreenLightDuration::new(255).unwrap().seconds(), 255);
    }

    #[test]
    fn from_str_valid() {
        assert_eq!(ScreenLightDuration::from_str("0").unwrap().seconds(), 0);
        assert_eq!(ScreenLightDuration::from_str("30").unwrap().seconds(), 30);
        assert_eq!(ScreenLightDuration::from_str("255").unwrap().seconds(), 255);
    }

    #[test]
    fn from_str_iso_duration() {
        assert_eq!(ScreenLightDuration::from_str("PT30S").unwrap().seconds(), 30);
        assert_eq!(ScreenLightDuration::from_str("PT1M").unwrap().seconds(), 60);
        assert_eq!(ScreenLightDuration::from_str("PT2M30S").unwrap().seconds(), 150);
        assert_eq!(ScreenLightDuration::from_str("PT0S").unwrap().seconds(), 0);
    }

    #[test]
    fn from_str_iso_duration_out_of_range() {
        assert!(ScreenLightDuration::from_str("PT5M").is_err());
        assert!(ScreenLightDuration::from_str("PT1H").is_err());
    }

    #[test]
    fn from_str_invalid() {
        assert!(ScreenLightDuration::from_str("256").is_err());
        assert!(ScreenLightDuration::from_str("-1").is_err());
        assert!(ScreenLightDuration::from_str("abc").is_err());
    }

    #[test]
    fn display() {
        assert_eq!(format!("{}", ScreenLightDuration::new(10).unwrap()), "10s");
        assert_eq!(format!("{}", ScreenLightDuration::new(0).unwrap()), "0s");
    }
}
