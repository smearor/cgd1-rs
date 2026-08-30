use std::fmt;
use std::fmt::Display;
use std::fmt::Formatter;
use std::str::FromStr;

use thiserror::Error;

/// Error parsing a [`Temperature`] from a string.
#[derive(Debug, Clone, PartialEq, Eq, Error)]
#[error("invalid temperature '{input}': {reason}")]
pub struct TemperatureParseError {
    /// The raw input string.
    pub input: String,
    /// The parse error reason.
    pub reason: String,
}

/// Temperature in degrees Celsius.
#[derive(Debug, Clone, Copy, PartialEq, PartialOrd)]
pub struct Temperature(f32);

impl Temperature {
    /// Create a new temperature value.
    pub fn new(value: f32) -> Self {
        Self(value)
    }

    /// Get the raw value in degrees Celsius.
    pub fn value(self) -> f32 {
        self.0
    }
}

impl From<f32> for Temperature {
    fn from(value: f32) -> Self {
        Self(value)
    }
}

impl Display for Temperature {
    fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.0)
    }
}

impl FromStr for Temperature {
    type Err = TemperatureParseError;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        let value: f32 = f32::from_str(s).map_err(|e| TemperatureParseError {
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
        let temp = Temperature::new(23.5);
        assert_eq!(temp.value(), 23.5);
    }

    #[test]
    fn from_f32() {
        let temp: Temperature = 10.0.into();
        assert_eq!(temp.value(), 10.0);
    }

    #[test]
    fn ordering() {
        let a = Temperature::new(10.0);
        let b = Temperature::new(20.0);
        assert!(a < b);
    }
}
