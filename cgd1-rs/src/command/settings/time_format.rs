use std::fmt;
use std::fmt::Display;
use std::fmt::Formatter;
use std::str::FromStr;

use crate::error::ClockError;
use crate::error::Result;
use thiserror::Error;

/// Error parsing a [`TimeFormat`] from a string.
#[derive(Debug, Clone, PartialEq, Eq, Error)]
#[error("invalid time format '{input}': {reason}")]
pub struct TimeFormatParseError {
    /// The raw input string.
    pub input: String,
    /// The parse error reason.
    pub reason: String,
}

/// Time display format for the CGD1.
///
/// Encoded as bit 1 of the flags byte in the settings payload.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TimeFormat {
    /// 24-hour format.
    TwentyFourHour,
    /// 12-hour format (AM/PM).
    TwelveHour,
}

impl TimeFormat {
    /// Encode as the flag bit value (0 = 24h, 1 = 12h).
    pub const fn flag_bit(self) -> u8 {
        match self {
            Self::TwentyFourHour => 0x00,
            Self::TwelveHour => 0x02,
        }
    }

    /// Decode from the raw flags byte.
    pub fn from_flags(flags: u8) -> Self {
        if flags & 0x02 != 0 { Self::TwelveHour } else { Self::TwentyFourHour }
    }
}

impl TryFrom<u8> for TimeFormat {
    type Error = ClockError;

    fn try_from(value: u8) -> Result<Self> {
        match value {
            0 => Ok(Self::TwentyFourHour),
            1 => Ok(Self::TwelveHour),
            other => Err(ClockError::Parse(format!("invalid time format: {other:#04x}"))),
        }
    }
}

impl From<TimeFormat> for u8 {
    fn from(format: TimeFormat) -> Self {
        match format {
            TimeFormat::TwentyFourHour => 0,
            TimeFormat::TwelveHour => 1,
        }
    }
}

impl Display for TimeFormat {
    fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
        match self {
            Self::TwentyFourHour => write!(f, "24"),
            Self::TwelveHour => write!(f, "12"),
        }
    }
}

impl FromStr for TimeFormat {
    type Err = TimeFormatParseError;

    fn from_str(s: &str) -> std::result::Result<Self, Self::Err> {
        match s {
            "12" => Ok(Self::TwelveHour),
            "24" => Ok(Self::TwentyFourHour),
            _ => Err(TimeFormatParseError {
                input: s.to_string(),
                reason: "must be '12' or '24'".to_string(),
            }),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn flag_bit_roundtrip() {
        assert_eq!(TimeFormat::TwentyFourHour.flag_bit(), 0x00);
        assert_eq!(TimeFormat::TwelveHour.flag_bit(), 0x02);
        assert_eq!(TimeFormat::from_flags(0x00), TimeFormat::TwentyFourHour);
        assert_eq!(TimeFormat::from_flags(0x02), TimeFormat::TwelveHour);
        assert_eq!(TimeFormat::from_flags(0x03), TimeFormat::TwelveHour);
    }

    #[test]
    fn try_from_u8() {
        assert_eq!(TimeFormat::try_from(0).unwrap(), TimeFormat::TwentyFourHour);
        assert_eq!(TimeFormat::try_from(1).unwrap(), TimeFormat::TwelveHour);
        assert!(TimeFormat::try_from(2).is_err());
    }

    #[test]
    fn into_u8() {
        let v: u8 = TimeFormat::TwentyFourHour.into();
        assert_eq!(v, 0);
        let v: u8 = TimeFormat::TwelveHour.into();
        assert_eq!(v, 1);
    }
}
