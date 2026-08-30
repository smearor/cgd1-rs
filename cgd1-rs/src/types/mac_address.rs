use std::fmt;
use std::fmt::Display;
use std::fmt::Formatter;
use std::str::FromStr;

use serde::Deserialize;
use serde::Serialize;
use thiserror::Error;

/// Error parsing a [`MacAddress`] from a string.
#[derive(Debug, Clone, PartialEq, Eq, Error)]
#[error("invalid MAC address '{input}': {reason}")]
pub struct MacAddressParseError {
    /// The raw input string.
    pub input: String,
    /// The parse error reason.
    pub reason: String,
}

/// A 6-byte BLE MAC address.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct MacAddress(#[serde(with = "mac_serde")] [u8; 6]);

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
    pub fn parse(s: &str) -> Result<Self, MacAddressParseError> {
        let normalized = normalized(s);
        if normalized.len() != 12 {
            return Err(MacAddressParseError {
                input: s.to_string(),
                reason: format!("invalid MAC address length: expected 12 hex chars, got {}", normalized.len()),
            });
        }
        let mut bytes = [0u8; 6];
        for i in 0..6 {
            bytes[i] = u8::from_str_radix(&normalized[i * 2..i * 2 + 2], 16).map_err(|e| MacAddressParseError {
                input: s.to_string(),
                reason: e.to_string(),
            })?;
        }
        Ok(Self(bytes))
    }

    pub fn normalized(&self) -> String {
        normalized(&self.to_string())
    }
}

impl Display for MacAddress {
    fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
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

impl FromStr for MacAddress {
    type Err = MacAddressParseError;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        Self::parse(s)
    }
}

pub(crate) fn normalized(s: &str) -> String {
    s.replace([':', '-'], "").to_lowercase()
}

mod mac_serde {
    use serde::Deserialize;
    use serde::Deserializer;
    use serde::Serializer;
    use std::str::FromStr;

    pub fn serialize<S>(bytes: &[u8; 6], serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        let s = format!("{:02x}:{:02x}:{:02x}:{:02x}:{:02x}:{:02x}", bytes[0], bytes[1], bytes[2], bytes[3], bytes[4], bytes[5]);
        serializer.serialize_str(&s)
    }

    pub fn deserialize<'de, D>(deserializer: D) -> Result<[u8; 6], D::Error>
    where
        D: Deserializer<'de>,
    {
        let s = String::deserialize(deserializer)?;
        let mac = crate::MacAddress::from_str(&s).map_err(serde::de::Error::custom)?;
        Ok(*mac.as_bytes())
    }
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
