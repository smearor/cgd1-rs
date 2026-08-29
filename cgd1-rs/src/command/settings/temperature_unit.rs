use crate::error::ClockError;
use crate::error::Result;

/// Temperature display unit for the CGD1.
///
/// Encoded as bit 2 of the flags byte in the settings payload.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TemperatureUnit {
    /// Degrees Celsius.
    Celsius,
    /// Degrees Fahrenheit.
    Fahrenheit,
}

impl TemperatureUnit {
    /// Encode as the flag bit value (0 = Celsius, 0x04 = Fahrenheit).
    pub const fn flag_bit(self) -> u8 {
        match self {
            Self::Celsius => 0x00,
            Self::Fahrenheit => 0x04,
        }
    }

    /// Decode from the raw flags byte.
    pub fn from_flags(flags: u8) -> Self {
        if flags & 0x04 != 0 { Self::Fahrenheit } else { Self::Celsius }
    }
}

impl TryFrom<u8> for TemperatureUnit {
    type Error = ClockError;

    fn try_from(value: u8) -> Result<Self> {
        match value {
            0 => Ok(Self::Celsius),
            1 => Ok(Self::Fahrenheit),
            other => Err(ClockError::Parse(format!("invalid temperature unit: {other:#04x}"))),
        }
    }
}

impl From<TemperatureUnit> for u8 {
    fn from(unit: TemperatureUnit) -> Self {
        match unit {
            TemperatureUnit::Celsius => 0,
            TemperatureUnit::Fahrenheit => 1,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn flag_bit_roundtrip() {
        assert_eq!(TemperatureUnit::Celsius.flag_bit(), 0x00);
        assert_eq!(TemperatureUnit::Fahrenheit.flag_bit(), 0x04);
        assert_eq!(TemperatureUnit::from_flags(0x00), TemperatureUnit::Celsius);
        assert_eq!(TemperatureUnit::from_flags(0x04), TemperatureUnit::Fahrenheit);
        assert_eq!(TemperatureUnit::from_flags(0x05), TemperatureUnit::Fahrenheit);
    }

    #[test]
    fn try_from_u8() {
        assert_eq!(TemperatureUnit::try_from(0).unwrap(), TemperatureUnit::Celsius);
        assert_eq!(TemperatureUnit::try_from(1).unwrap(), TemperatureUnit::Fahrenheit);
        assert!(TemperatureUnit::try_from(2).is_err());
    }

    #[test]
    fn into_u8() {
        let v: u8 = TemperatureUnit::Celsius.into();
        assert_eq!(v, 0);
        let v: u8 = TemperatureUnit::Fahrenheit.into();
        assert_eq!(v, 1);
    }
}
