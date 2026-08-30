use std::fmt;
use std::fmt::Display;
use std::fmt::Formatter;
use std::str::FromStr;

use crate::error::ClockError;
use crate::error::Result;
use thiserror::Error;

/// Error parsing a [`ClockTime`] from a string.
#[derive(Debug, Clone, PartialEq, Eq, Error)]
#[error("invalid clock time '{input}': {reason}")]
pub struct ClockTimeParseError {
    /// The raw input string.
    pub input: String,
    /// The parse error reason.
    pub reason: String,
}

/// A time of day (hour 0–23, minute 0–59).
///
/// Used by alarm entries and night-mode settings to represent
/// a specific time of day in 24-hour format.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct ClockTime {
    hour: u8,
    minute: u8,
}

impl ClockTime {
    /// Create a validated clock time. Hour must be 0–23, minute 0–59.
    pub fn new(hour: u8, minute: u8) -> Result<Self> {
        if hour > 23 {
            return Err(ClockError::Parse(format!("invalid hour: {hour}")));
        }
        if minute > 59 {
            return Err(ClockError::Parse(format!("invalid minute: {minute}")));
        }
        Ok(Self { hour, minute })
    }

    /// Get the hour (0–23).
    pub const fn hour(self) -> u8 {
        self.hour
    }

    /// Get the minute (0–59).
    pub const fn minute(self) -> u8 {
        self.minute
    }
}

impl Display for ClockTime {
    fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
        write!(f, "{:02}:{:02}", self.hour, self.minute)
    }
}

impl FromStr for ClockTime {
    type Err = ClockTimeParseError;

    fn from_str(s: &str) -> std::result::Result<Self, Self::Err> {
        let parts: Vec<&str> = s.split(':').collect();
        if parts.len() != 2 {
            return Err(ClockTimeParseError {
                input: s.to_string(),
                reason: "expected format HH:MM".to_string(),
            });
        }
        let hour: u8 = u8::from_str(parts[0]).map_err(|e| ClockTimeParseError {
            input: s.to_string(),
            reason: format!("invalid hour: {e}"),
        })?;
        let minute: u8 = u8::from_str(parts[1]).map_err(|e| ClockTimeParseError {
            input: s.to_string(),
            reason: format!("invalid minute: {e}"),
        })?;
        Self::new(hour, minute).map_err(|e| ClockTimeParseError {
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
        let t = ClockTime::new(7, 30).unwrap();
        assert_eq!(t.hour(), 7);
        assert_eq!(t.minute(), 30);
    }

    #[test]
    fn new_boundary_values() {
        assert!(ClockTime::new(0, 0).is_ok());
        assert!(ClockTime::new(23, 59).is_ok());
    }

    #[test]
    fn new_rejects_invalid_hour() {
        assert!(ClockTime::new(24, 0).is_err());
        assert!(ClockTime::new(255, 0).is_err());
    }

    #[test]
    fn new_rejects_invalid_minute() {
        assert!(ClockTime::new(0, 60).is_err());
        assert!(ClockTime::new(0, 255).is_err());
    }

    #[test]
    fn from_str_valid() {
        let t = ClockTime::from_str("07:30").unwrap();
        assert_eq!(t.hour(), 7);
        assert_eq!(t.minute(), 30);
    }

    #[test]
    fn from_str_boundary() {
        assert!(ClockTime::from_str("00:00").is_ok());
        assert!(ClockTime::from_str("23:59").is_ok());
    }

    #[test]
    fn from_str_invalid_format() {
        assert!(ClockTime::from_str("730").is_err());
        assert!(ClockTime::from_str("7:30:00").is_err());
        assert!(ClockTime::from_str("").is_err());
    }

    #[test]
    fn from_str_out_of_range() {
        assert!(ClockTime::from_str("24:00").is_err());
        assert!(ClockTime::from_str("12:60").is_err());
    }

    #[test]
    fn display() {
        assert_eq!(format!("{}", ClockTime::new(7, 30).unwrap()), "07:30");
        assert_eq!(format!("{}", ClockTime::new(0, 0).unwrap()), "00:00");
        assert_eq!(format!("{}", ClockTime::new(23, 59).unwrap()), "23:59");
    }
}
