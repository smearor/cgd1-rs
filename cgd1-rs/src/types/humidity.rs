use std::fmt;
use std::fmt::Display;
use std::fmt::Formatter;
use std::str::FromStr;

use thiserror::Error;

/// Error parsing a [`Humidity`] from a string.
#[derive(Debug, Clone, PartialEq, Eq, Error)]
#[error("invalid humidity '{input}': {reason}")]
pub struct HumidityParseError {
    /// The raw input string.
    pub input: String,
    /// The parse error reason.
    pub reason: String,
}

/// Relative humidity in percent.
#[derive(Debug, Clone, Copy, PartialEq, PartialOrd)]
pub struct Humidity(f32);

impl Humidity {
    /// Create a new humidity value.
    pub fn new(value: f32) -> Self {
        Self(value)
    }

    /// Get the raw value in percent.
    pub fn value(self) -> f32 {
        self.0
    }
}

impl From<f32> for Humidity {
    fn from(value: f32) -> Self {
        Self(value)
    }
}

impl Display for Humidity {
    fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.0)
    }
}

impl FromStr for Humidity {
    type Err = HumidityParseError;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        let value: f32 = f32::from_str(s).map_err(|e| HumidityParseError {
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
        let hum = Humidity::new(45.5);
        assert_eq!(hum.value(), 45.5);
    }

    #[test]
    fn from_f32() {
        let hum: Humidity = 50.0.into();
        assert_eq!(hum.value(), 50.0);
    }

    #[test]
    fn ordering() {
        let a = Humidity::new(30.0);
        let b = Humidity::new(60.0);
        assert!(a < b);
    }
}
