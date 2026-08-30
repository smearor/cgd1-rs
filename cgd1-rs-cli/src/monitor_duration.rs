use std::fmt;
use std::fmt::Display;
use std::fmt::Formatter;
use std::str::FromStr;

use cgd1_rs::parse_iso_duration_to_seconds;
use thiserror::Error;

/// Error parsing a [`MonitorDuration`] from a string.
#[derive(Debug, Clone, PartialEq, Eq, Error)]
#[error("invalid monitor duration '{input}': {reason}")]
pub struct MonitorDurationParseError {
    /// The raw input string.
    pub input: String,
    /// The parse error reason.
    pub reason: String,
}

/// Monitor duration in seconds (0 = indefinite).
///
/// Used by the `monitor` CLI command to limit sensor data monitoring time.
/// A value of 0 means monitor indefinitely until interrupted (Ctrl+C).
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct MonitorDuration(u64);

impl MonitorDuration {
    /// Indefinite monitoring (0 seconds = run until interrupted).
    pub const INDEFINITE: MonitorDuration = MonitorDuration(0);

    /// Create a monitor duration. 0 means indefinite, any positive value
    /// is the duration in seconds.
    pub fn new(seconds: u64) -> std::result::Result<Self, MonitorDurationParseError> {
        Ok(Self(seconds))
    }

    /// Get the duration in seconds. 0 means indefinite.
    pub const fn seconds(self) -> u64 {
        self.0
    }

    /// Whether this duration is indefinite (0 seconds).
    pub const fn is_indefinite(self) -> bool {
        self.0 == 0
    }
}

impl TryFrom<u64> for MonitorDuration {
    type Error = MonitorDurationParseError;

    fn try_from(seconds: u64) -> Result<Self, Self::Error> {
        Self::new(seconds)
    }
}

impl From<MonitorDuration> for u64 {
    fn from(d: MonitorDuration) -> Self {
        d.0
    }
}

impl From<MonitorDuration> for std::time::Duration {
    fn from(d: MonitorDuration) -> Self {
        std::time::Duration::from_secs(d.0)
    }
}

impl Display for MonitorDuration {
    fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
        if self.is_indefinite() {
            write!(f, "indefinite")
        } else {
            write!(f, "{}s", self.0)
        }
    }
}

impl FromStr for MonitorDuration {
    type Err = MonitorDurationParseError;

    fn from_str(s: &str) -> std::result::Result<Self, Self::Err> {
        let seconds: u64 = if s.starts_with('P') {
            parse_iso_duration_to_seconds(s).ok_or_else(|| MonitorDurationParseError {
                input: s.to_string(),
                reason: "invalid ISO 8601 duration".to_string(),
            })?
        } else {
            u64::from_str(s).map_err(|e| MonitorDurationParseError {
                input: s.to_string(),
                reason: e.to_string(),
            })?
        };
        Self::new(seconds)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn new_valid() {
        assert_eq!(MonitorDuration::new(0).unwrap().seconds(), 0);
        assert_eq!(MonitorDuration::new(60).unwrap().seconds(), 60);
        assert_eq!(MonitorDuration::new(3600).unwrap().seconds(), 3600);
    }

    #[test]
    fn is_indefinite() {
        assert!(MonitorDuration::INDEFINITE.is_indefinite());
        assert!(MonitorDuration::new(0).unwrap().is_indefinite());
        assert!(!MonitorDuration::new(60).unwrap().is_indefinite());
    }

    #[test]
    fn from_str_valid() {
        assert_eq!(MonitorDuration::from_str("0").unwrap().seconds(), 0);
        assert_eq!(MonitorDuration::from_str("60").unwrap().seconds(), 60);
    }

    #[test]
    fn from_str_iso_duration() {
        assert_eq!(MonitorDuration::from_str("PT30S").unwrap().seconds(), 30);
        assert_eq!(MonitorDuration::from_str("PT1M").unwrap().seconds(), 60);
        assert_eq!(MonitorDuration::from_str("PT1H").unwrap().seconds(), 3600);
        assert_eq!(MonitorDuration::from_str("PT1H30M").unwrap().seconds(), 5400);
    }

    #[test]
    fn from_str_invalid() {
        assert!(MonitorDuration::from_str("-1").is_err());
        assert!(MonitorDuration::from_str("abc").is_err());
    }

    #[test]
    fn display() {
        assert_eq!(format!("{}", MonitorDuration::new(0).unwrap()), "indefinite");
        assert_eq!(format!("{}", MonitorDuration::new(60).unwrap()), "60s");
    }

    #[test]
    fn into_std_duration() {
        let d: std::time::Duration = MonitorDuration::new(60).unwrap().into();
        assert_eq!(d, std::time::Duration::from_secs(60));
    }
}
