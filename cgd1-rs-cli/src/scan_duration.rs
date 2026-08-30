use std::fmt;
use std::fmt::Display;
use std::fmt::Formatter;
use std::str::FromStr;

use cgd1_rs::parse_iso_duration_to_seconds;
use thiserror::Error;

/// Error parsing a [`ScanDuration`] from a string.
#[derive(Debug, Clone, PartialEq, Eq, Error)]
#[error("invalid scan duration '{input}': {reason}")]
pub struct ScanDurationParseError {
    /// The raw input string.
    pub input: String,
    /// The parse error reason.
    pub reason: String,
}

/// Scan duration in seconds (1–600).
///
/// Used by the `scan` CLI command to limit BLE discovery time.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct ScanDuration(u64);

impl ScanDuration {
    /// Minimum scan duration in seconds.
    pub const MIN: ScanDuration = ScanDuration(1);
    /// Maximum scan duration in seconds.
    pub const MAX: ScanDuration = ScanDuration(600);

    /// Create a validated scan duration. Must be 1–600 seconds.
    pub fn new(seconds: u64) -> Result<Self, ScanDurationParseError> {
        if seconds < Self::MIN.0 {
            return Err(ScanDurationParseError {
                input: seconds.to_string(),
                reason: "must be at least 1 second".to_string(),
            });
        }
        if seconds > Self::MAX.0 {
            return Err(ScanDurationParseError {
                input: seconds.to_string(),
                reason: "must be at most 600 seconds (10 minutes)".to_string(),
            });
        }
        Ok(Self(seconds))
    }

    /// Get the duration in seconds.
    pub const fn seconds(self) -> u64 {
        self.0
    }
}

impl TryFrom<u64> for ScanDuration {
    type Error = ScanDurationParseError;

    fn try_from(seconds: u64) -> Result<Self, Self::Error> {
        Self::new(seconds)
    }
}

impl From<ScanDuration> for u64 {
    fn from(d: ScanDuration) -> Self {
        d.0
    }
}

impl From<ScanDuration> for std::time::Duration {
    fn from(d: ScanDuration) -> Self {
        std::time::Duration::from_secs(d.0)
    }
}

impl Display for ScanDuration {
    fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
        write!(f, "{}s", self.0)
    }
}

impl FromStr for ScanDuration {
    type Err = ScanDurationParseError;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        let seconds: u64 = if s.starts_with('P') {
            parse_iso_duration_to_seconds(s).ok_or_else(|| ScanDurationParseError {
                input: s.to_string(),
                reason: "invalid ISO 8601 duration".to_string(),
            })?
        } else {
            u64::from_str(s).map_err(|e| ScanDurationParseError {
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
        assert_eq!(ScanDuration::new(1).unwrap().seconds(), 1);
        assert_eq!(ScanDuration::new(10).unwrap().seconds(), 10);
        assert_eq!(ScanDuration::new(600).unwrap().seconds(), 600);
    }

    #[test]
    fn new_rejects_zero() {
        assert!(ScanDuration::new(0).is_err());
    }

    #[test]
    fn new_rejects_too_large() {
        assert!(ScanDuration::new(601).is_err());
    }

    #[test]
    fn from_str_valid() {
        assert_eq!(ScanDuration::from_str("30").unwrap().seconds(), 30);
    }

    #[test]
    fn from_str_iso_duration() {
        assert_eq!(ScanDuration::from_str("PT30S").unwrap().seconds(), 30);
        assert_eq!(ScanDuration::from_str("PT1M").unwrap().seconds(), 60);
        assert_eq!(ScanDuration::from_str("PT5M").unwrap().seconds(), 300);
        assert_eq!(ScanDuration::from_str("PT10M").unwrap().seconds(), 600);
    }

    #[test]
    fn from_str_iso_duration_out_of_range() {
        assert!(ScanDuration::from_str("PT10M1S").is_err());
        assert!(ScanDuration::from_str("PT1H").is_err());
    }

    #[test]
    fn from_str_invalid() {
        assert!(ScanDuration::from_str("0").is_err());
        assert!(ScanDuration::from_str("601").is_err());
        assert!(ScanDuration::from_str("abc").is_err());
    }

    #[test]
    fn display() {
        assert_eq!(format!("{}", ScanDuration::new(30).unwrap()), "30s");
    }

    #[test]
    fn into_std_duration() {
        let d: std::time::Duration = ScanDuration::new(30).unwrap().into();
        assert_eq!(d, std::time::Duration::from_secs(30));
    }
}
