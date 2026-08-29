use std::fmt;

use crate::error::ClockError;
use crate::error::Result;

/// A 6-byte BLE MAC address.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct MacAddress([u8; 6]);

impl MacAddress {
    /// Create a new MAC address from raw bytes.
    pub fn new(bytes: [u8; 6]) -> Self {
        Self(bytes)
    }

    /// Get the raw bytes.
    pub fn as_bytes(&self) -> &[u8; 6] {
        &self.0
    }

    /// Parse a MAC address from a colon-separated string (e.g. `aa:bb:cc:dd:ee:ff`).
    pub fn parse(s: &str) -> Result<Self> {
        let normalized = normalized(s);
        if normalized.len() != 12 {
            return Err(ClockError::Parse(format!("invalid MAC address length: {s}")));
        }
        let mut bytes = [0u8; 6];
        for i in 0..6 {
            bytes[i] = u8::from_str_radix(&normalized[i * 2..i * 2 + 2], 16).map_err(|_| ClockError::Parse(format!("invalid MAC address: {s}")))?;
        }
        Ok(Self(bytes))
    }

    pub fn normalized(&self) -> String {
        normalized(&self.to_string())
    }
}

impl fmt::Display for MacAddress {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            f,
            "{:02x}:{:02x}:{:02x}:{:02x}:{:02x}:{:02x}",
            self.0[0], self.0[1], self.0[2], self.0[3], self.0[4], self.0[5]
        )
    }
}

impl From<[u8; 6]> for MacAddress {
    fn from(bytes: [u8; 6]) -> Self {
        Self(bytes)
    }
}

pub(crate) fn normalized(s: &str) -> String {
    s.replace([':', '-'], "").to_lowercase()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parse_valid() {
        let mac = MacAddress::parse("aa:bb:cc:dd:ee:ff").unwrap();
        assert_eq!(mac.as_bytes(), &[0xaa, 0xbb, 0xcc, 0xdd, 0xee, 0xff]);
    }

    #[test]
    fn parse_with_dashes() {
        let mac = MacAddress::parse("AA-BB-CC-DD-EE-FF").unwrap();
        assert_eq!(mac.as_bytes(), &[0xaa, 0xbb, 0xcc, 0xdd, 0xee, 0xff]);
    }

    #[test]
    fn parse_too_short() {
        assert!(MacAddress::parse("aa:bb").is_err());
    }

    #[test]
    fn to_string_roundtrip() {
        let mac = MacAddress::new([0xff, 0xee, 0xdd, 0xcc, 0xbb, 0xaa]);
        let s = mac.to_string();
        let parsed = MacAddress::parse(&s).unwrap();
        assert_eq!(mac, parsed);
    }

    #[test]
    fn display_format() {
        let mac = MacAddress::new([0x01, 0x02, 0x03, 0x04, 0x05, 0x06]);
        assert_eq!(format!("{mac}"), "01:02:03:04:05:06");
    }
}
