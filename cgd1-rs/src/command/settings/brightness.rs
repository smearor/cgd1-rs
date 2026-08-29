use crate::error::ClockError;
use crate::error::Result;

/// Brightness level (0–150, must be a multiple of 10).
///
/// Used for both daytime and nighttime brightness in [`super::DeviceSettings`].
/// Encoded as a nibble value (0–15) in the packed brightness byte.
/// The device accepts nibble values 0–15, though typical range is 0–10 (0–100%).
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub struct Brightness(u8);

impl Brightness {
    /// Minimum brightness.
    pub const MIN: Brightness = Brightness(0);
    /// Maximum brightness (nibble 15 × 10).
    pub const MAX: Brightness = Brightness(150);

    /// Create a validated brightness value. Must be 0–150 and a multiple of 10.
    pub fn new(value: u8) -> Result<Self> {
        if value > 150 {
            return Err(ClockError::InvalidSettings(format!("brightness {value} out of range 0-150")));
        }
        if !value.is_multiple_of(10) {
            return Err(ClockError::InvalidSettings(format!("brightness {value} must be a multiple of 10")));
        }
        Ok(Self(value))
    }

    /// Get the raw brightness value (0–150).
    pub const fn value(self) -> u8 {
        self.0
    }

    /// Get the nibble-encoded value (0–15) for the packed brightness byte.
    pub const fn nibble(self) -> u8 {
        self.0 / 10
    }

    /// Decode a nibble value (0–15) back into a Brightness.
    pub fn from_nibble(nibble: u8) -> Result<Self> {
        Self::new(nibble * 10)
    }
}

impl TryFrom<u8> for Brightness {
    type Error = ClockError;

    fn try_from(value: u8) -> Result<Self> {
        Self::new(value)
    }
}

impl From<Brightness> for u8 {
    fn from(b: Brightness) -> Self {
        b.0
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn new_valid() {
        assert_eq!(Brightness::new(0).unwrap().value(), 0);
        assert_eq!(Brightness::new(100).unwrap().value(), 100);
        assert_eq!(Brightness::new(150).unwrap().value(), 150);
        assert_eq!(Brightness::new(50).unwrap().value(), 50);
    }

    #[test]
    fn new_rejects_out_of_range() {
        assert!(Brightness::new(151).is_err());
        assert!(Brightness::new(255).is_err());
    }

    #[test]
    fn new_rejects_non_multiple_of_10() {
        assert!(Brightness::new(5).is_err());
        assert!(Brightness::new(33).is_err());
        assert!(Brightness::new(99).is_err());
    }

    #[test]
    fn nibble_roundtrip() {
        assert_eq!(Brightness::new(0).unwrap().nibble(), 0);
        assert_eq!(Brightness::new(100).unwrap().nibble(), 10);
        assert_eq!(Brightness::new(150).unwrap().nibble(), 15);
        assert_eq!(Brightness::new(80).unwrap().nibble(), 8);
        assert_eq!(Brightness::from_nibble(8).unwrap().value(), 80);
        assert_eq!(Brightness::from_nibble(0).unwrap().value(), 0);
        assert_eq!(Brightness::from_nibble(10).unwrap().value(), 100);
        assert_eq!(Brightness::from_nibble(15).unwrap().value(), 150);
    }

    #[test]
    fn from_nibble_rejects_invalid() {
        assert!(Brightness::from_nibble(16).is_err());
    }
}
