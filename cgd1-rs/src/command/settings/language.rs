use crate::error::ClockError;
use crate::error::Result;

/// Display language for the CGD1.
///
/// Encoded as bit 0 of the flags byte in the settings payload.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Language {
    /// Chinese (Simplified).
    Chinese,
    /// English.
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
