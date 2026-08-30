use std::fmt;
use std::fmt::Display;
use std::fmt::Formatter;
use std::str::FromStr;

use crate::error::ClockError;
use crate::error::Result;
use thiserror::Error;

/// Error parsing an [`AlarmSlotIndex`] from a string.
#[derive(Debug, Clone, PartialEq, Eq, Error)]
#[error("invalid alarm slot index '{input}': {reason}")]
pub struct AlarmSlotIndexParseError {
    /// The raw input string.
    pub input: String,
    /// The parse error reason.
    pub reason: String,
}

/// Index of an alarm slot on the CGD1 device.
///
/// Valid range: 0–15 (16 slots total). Construction validates
/// the range, so any `AlarmSlotIndex` is guaranteed valid.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct AlarmSlotIndex(u8);

impl AlarmSlotIndex {
    /// Maximum valid slot index.
    pub const MAX: u8 = 15;

    /// Create a slot index from a raw `u8`, validating the range.
    pub fn new(index: u8) -> Result<Self> {
        if index > Self::MAX {
            return Err(ClockError::Parse(format!("invalid alarm slot: {index}")));
        }
        Ok(Self(index))
    }

    /// Get the raw slot index value.
    pub const fn value(self) -> u8 {
        self.0
    }
}

impl TryFrom<u8> for AlarmSlotIndex {
    type Error = ClockError;

    fn try_from(index: u8) -> Result<Self> {
        Self::new(index)
    }
}

impl From<AlarmSlotIndex> for u8 {
    fn from(index: AlarmSlotIndex) -> u8 {
        index.0
    }
}

impl Display for AlarmSlotIndex {
    fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.0)
    }
}

impl FromStr for AlarmSlotIndex {
    type Err = AlarmSlotIndexParseError;

    fn from_str(s: &str) -> std::result::Result<Self, Self::Err> {
        let index: u8 = u8::from_str(s).map_err(|e| AlarmSlotIndexParseError {
            input: s.to_string(),
            reason: e.to_string(),
        })?;
        if index > Self::MAX {
            return Err(AlarmSlotIndexParseError {
                input: s.to_string(),
                reason: format!("must be 0–{}", Self::MAX),
            });
        }
        Ok(Self(index))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn valid_index() {
        let idx = AlarmSlotIndex::new(0).unwrap();
        assert_eq!(idx.value(), 0);
        let idx = AlarmSlotIndex::new(15).unwrap();
        assert_eq!(idx.value(), 15);
    }

    #[test]
    fn invalid_index() {
        assert!(AlarmSlotIndex::new(16).is_err());
        assert!(AlarmSlotIndex::new(255).is_err());
    }

    #[test]
    fn try_from_valid() {
        let idx: AlarmSlotIndex = 10u8.try_into().unwrap();
        assert_eq!(idx.value(), 10);
    }

    #[test]
    fn try_from_invalid() {
        let result: Result<AlarmSlotIndex> = 16u8.try_into();
        assert!(result.is_err());
    }

    #[test]
    fn into_u8() {
        let idx = AlarmSlotIndex::new(7).unwrap();
        let raw: u8 = idx.into();
        assert_eq!(raw, 7);
    }
}
