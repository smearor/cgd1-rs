use std::fmt;
use std::fmt::Display;
use std::fmt::Formatter;
use std::str::FromStr;

use crate::error::ClockError;
use crate::error::Result;
use thiserror::Error;

/// Error parsing a [`Volume`] from a string.
#[derive(Debug, Clone, PartialEq, Eq, Error)]
#[error("invalid volume '{input}': {reason}")]
pub struct VolumeParseError {
    /// The raw input string.
    pub input: String,
    /// The parse error reason.
    pub reason: String,
}

/// Sound volume level (1–5).
///
/// Encoded as byte 0 of the settings payload.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct Volume(u8);

impl Volume {
    /// Minimum volume level.
    pub const MIN: Volume = Volume(1);
    /// Maximum volume level.
    pub const MAX: Volume = Volume(5);

    /// Create a validated volume value. Must be 1–5.
    pub fn new(value: u8) -> Result<Self> {
        if !(1..=5).contains(&value) {
            return Err(ClockError::InvalidSettings(format!("volume {value} out of range 1-5")));
        }
        Ok(Self(value))
    }

    /// Get the raw volume value (1–5).
    pub const fn value(self) -> u8 {
        self.0
    }
}

impl TryFrom<u8> for Volume {
    type Error = ClockError;

    fn try_from(value: u8) -> Result<Self> {
        Self::new(value)
    }
}

impl From<Volume> for u8 {
    fn from(v: Volume) -> Self {
        v.0
    }
}

impl Display for Volume {
    fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.0)
    }
}

impl FromStr for Volume {
    type Err = VolumeParseError;

    fn from_str(s: &str) -> std::result::Result<Self, Self::Err> {
        let value: u8 = u8::from_str(s).map_err(|e| VolumeParseError {
            input: s.to_string(),
            reason: e.to_string(),
        })?;
        Self::new(value).map_err(|e| VolumeParseError {
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
        assert_eq!(Volume::new(1).unwrap().value(), 1);
        assert_eq!(Volume::new(3).unwrap().value(), 3);
        assert_eq!(Volume::new(5).unwrap().value(), 5);
    }

    #[test]
    fn new_rejects_out_of_range() {
        assert!(Volume::new(0).is_err());
        assert!(Volume::new(6).is_err());
        assert!(Volume::new(255).is_err());
    }

    #[test]
    fn from_str_valid() {
        assert_eq!(Volume::from_str("3").unwrap().value(), 3);
    }

    #[test]
    fn from_str_invalid() {
        assert!(Volume::from_str("0").is_err());
        assert!(Volume::from_str("6").is_err());
        assert!(Volume::from_str("abc").is_err());
    }

    #[test]
    fn display() {
        assert_eq!(format!("{}", Volume::new(3).unwrap()), "3");
    }
}
