use std::fmt;
use std::fmt::Display;
use std::fmt::Formatter;
use std::str::FromStr;

use crate::error::ClockError;
use crate::error::Result;
use serde::Deserialize;
use serde::Serialize;
use thiserror::Error;

/// Error parsing a [`Language`] from a string.
#[derive(Debug, Clone, PartialEq, Eq, Error)]
#[error("invalid language '{input}': {reason}")]
pub struct LanguageParseError {
    /// The raw input string.
    pub input: String,
    /// The parse error reason.
    pub reason: String,
}

/// Display language for the CGD1.
///
/// Encoded as bit 0 of the flags byte in the settings payload.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum Language {
    /// Chinese (Simplified).
    #[serde(rename = "zh")]
    Chinese,
    /// English.
    #[serde(rename = "en")]
    English,
}

impl Language {
    /// Encode as the flag bit value (0 = Chinese, 0x01 = English).
    pub const fn flag_bit(self) -> u8 {
        match self {
            Self::Chinese => 0x00,
            Self::English => 0x01,
        }
    }

    /// Decode from the raw flags byte.
    pub fn from_flags(flags: u8) -> Self {
        if flags & 0x01 != 0 { Self::English } else { Self::Chinese }
    }
}

impl TryFrom<u8> for Language {
    type Error = ClockError;

    fn try_from(value: u8) -> Result<Self> {
        match value {
            0 => Ok(Self::Chinese),
            1 => Ok(Self::English),
            other => Err(ClockError::Parse(format!("invalid language: {other:#04x}"))),
        }
    }
}

impl From<Language> for u8 {
    fn from(lang: Language) -> Self {
        match lang {
            Language::Chinese => 0,
            Language::English => 1,
        }
    }
}

impl Display for Language {
    fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
        match self {
            Self::Chinese => write!(f, "zh"),
            Self::English => write!(f, "en"),
        }
    }
}

impl FromStr for Language {
    type Err = LanguageParseError;

    fn from_str(s: &str) -> std::result::Result<Self, Self::Err> {
        match s.to_ascii_lowercase().as_str() {
            "en" | "english" => Ok(Self::English),
            "zh" | "chinese" => Ok(Self::Chinese),
            _ => Err(LanguageParseError {
                input: s.to_string(),
                reason: "must be 'en' or 'zh'".to_string(),
            }),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn flag_bit_roundtrip() {
        assert_eq!(Language::Chinese.flag_bit(), 0x00);
        assert_eq!(Language::English.flag_bit(), 0x01);
        assert_eq!(Language::from_flags(0x00), Language::Chinese);
        assert_eq!(Language::from_flags(0x01), Language::English);
        assert_eq!(Language::from_flags(0x03), Language::English);
    }

    #[test]
    fn try_from_u8() {
        assert_eq!(Language::try_from(0).unwrap(), Language::Chinese);
        assert_eq!(Language::try_from(1).unwrap(), Language::English);
        assert!(Language::try_from(2).is_err());
    }

    #[test]
    fn into_u8() {
        let v: u8 = Language::Chinese.into();
        assert_eq!(v, 0);
        let v: u8 = Language::English.into();
        assert_eq!(v, 1);
    }
}
