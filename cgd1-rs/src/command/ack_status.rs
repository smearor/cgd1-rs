use std::fmt;
use std::fmt::Display;
use std::fmt::Formatter;

/// ACK status byte from a CGD1 BLE notification.
///
/// The protocol defines only two outcomes:
/// - `0x00` = Success
/// - Any other value = Failure (command rejected)
///
/// The raw byte is preserved in the [`AckStatus::Failure`] variant for
/// diagnostics, since the protocol does not standardize failure codes.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum AckStatus {
    /// Command succeeded (status byte `0x00`).
    Success,
    /// Command failed (non-zero status byte).
    Failure(u8),
}

impl AckStatus {
    /// Create an `AckStatus` from a raw status byte.
    pub fn from_byte(byte: u8) -> Self {
        if byte == 0x00 { Self::Success } else { Self::Failure(byte) }
    }

    /// Whether the status indicates success.
    pub fn is_success(&self) -> bool {
        *self == Self::Success
    }

    /// Convert back to the raw status byte.
    pub fn as_byte(&self) -> u8 {
        match self {
            Self::Success => 0x00,
            Self::Failure(byte) => *byte,
        }
    }
}

impl From<u8> for AckStatus {
    fn from(byte: u8) -> Self {
        Self::from_byte(byte)
    }
}

impl From<AckStatus> for u8 {
    fn from(status: AckStatus) -> Self {
        status.as_byte()
    }
}

impl Display for AckStatus {
    fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
        match self {
            Self::Success => write!(f, "success"),
            Self::Failure(code) => write!(f, "failure(0x{code:02x})"),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn success_from_zero() {
        assert_eq!(AckStatus::from_byte(0x00), AckStatus::Success);
        assert!(AckStatus::Success.is_success());
    }

    #[test]
    fn failure_from_nonzero() {
        assert_eq!(AckStatus::from_byte(0x01), AckStatus::Failure(0x01));
        assert!(!AckStatus::Failure(0x01).is_success());
    }

    #[test]
    fn as_byte_roundtrip() {
        assert_eq!(AckStatus::Success.as_byte(), 0x00);
        assert_eq!(AckStatus::Failure(0x09).as_byte(), 0x09);
    }

    #[test]
    fn from_u8_conversion() {
        let status: AckStatus = 0x00u8.into();
        assert_eq!(status, AckStatus::Success);
        let status: AckStatus = 0x01u8.into();
        assert_eq!(status, AckStatus::Failure(0x01));
    }

    #[test]
    fn into_u8_conversion() {
        let byte: u8 = AckStatus::Success.into();
        assert_eq!(byte, 0x00);
        let byte: u8 = AckStatus::Failure(0x09).into();
        assert_eq!(byte, 0x09);
    }
}
