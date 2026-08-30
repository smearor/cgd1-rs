use std::fmt;
use std::fmt::Display;
use std::fmt::Formatter;
use std::str::FromStr;

use thiserror::Error;

/// Error parsing a [`BatteryLevel`] from a string.
#[derive(Debug, Clone, PartialEq, Eq, Error)]
#[error("invalid battery level '{input}': {reason}")]
pub struct BatteryLevelParseError {
    /// The raw input string.
    pub input: String,
    /// The parse error reason.
    pub reason: String,
}

/// Battery level percentage (0-100).
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct BatteryLevel(u8);

impl BatteryLevel {
    /// Create a new battery level value.
    pub fn new(value: u8) -> Self {
        Self(value)
    }

    /// Get the raw percentage value.
    pub fn value(self) -> u8 {
        self.0
    }
}

impl From<u8> for BatteryLevel {
    fn from(value: u8) -> Self {
        Self(value)
    }
}

impl Display for BatteryLevel {
    fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.0)
    }
}

impl FromStr for BatteryLevel {
    type Err = BatteryLevelParseError;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        let value: u8 = u8::from_str(s).map_err(|e| BatteryLevelParseError {
            input: s.to_string(),
            reason: e.to_string(),
        })?;
        Ok(Self(value))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn new_and_value() {
        let level = BatteryLevel::new(90);
        assert_eq!(level.value(), 90);
    }

    #[test]
    fn from_u8() {
        let level: BatteryLevel = 50.into();
        assert_eq!(level.value(), 50);
    }

    #[test]
    fn ordering() {
        let a = BatteryLevel::new(10);
        let b = BatteryLevel::new(90);
        assert!(a < b);
    }
}
