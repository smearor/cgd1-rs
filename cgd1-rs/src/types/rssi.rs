use std::fmt;
use std::fmt::Display;
use std::fmt::Formatter;
use std::str::FromStr;

use serde::Deserialize;
use serde::Serialize;
use thiserror::Error;

/// Error parsing an [`Rssi`] from a string.
#[derive(Debug, Clone, PartialEq, Eq, Error)]
#[error("invalid RSSI '{input}': {reason}")]
pub struct RssiParseError {
    /// The raw input string.
    pub input: String,
    /// The parse error reason.
    pub reason: String,
}

/// RSSI signal strength in dBm (typically -100 to 0).
///
/// Represents the received signal strength indicator from a BLE advertisement.
/// Values closer to 0 indicate a stronger signal.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
#[serde(transparent)]
pub struct Rssi(i16);

impl Rssi {
    /// Create a new RSSI value from a raw dBm reading.
    pub const fn new(dbm: i16) -> Self {
        Self(dbm)
    }

    /// Get the RSSI value in dBm.
    pub const fn dbm(self) -> i16 {
        self.0
    }
}

impl From<i16> for Rssi {
    fn from(dbm: i16) -> Self {
        Self(dbm)
    }
}

impl From<Rssi> for i16 {
    fn from(r: Rssi) -> Self {
        r.0
    }
}

impl Display for Rssi {
    fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
        write!(f, "{} dBm", self.0)
    }
}

impl FromStr for Rssi {
    type Err = RssiParseError;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        let s_trimmed = s.trim().trim_end_matches("dBm").trim();
        i16::from_str(s_trimmed).map(Self).map_err(|e| RssiParseError {
            input: s.to_string(),
            reason: e.to_string(),
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn new_valid() {
        assert_eq!(Rssi::new(-50).dbm(), -50);
        assert_eq!(Rssi::new(0).dbm(), 0);
        assert_eq!(Rssi::new(-100).dbm(), -100);
    }

    #[test]
    fn from_i16() {
        let r: Rssi = (-42).into();
        assert_eq!(r.dbm(), -42);
    }

    #[test]
    fn into_i16() {
        let r = Rssi::new(-60);
        let v: i16 = r.into();
        assert_eq!(v, -60);
    }

    #[test]
    fn display() {
        assert_eq!(format!("{}", Rssi::new(-50)), "-50 dBm");
    }

    #[test]
    fn from_str_valid() {
        assert_eq!(Rssi::from_str("-50").unwrap().dbm(), -50);
        assert_eq!(Rssi::from_str("-50 dBm").unwrap().dbm(), -50);
        assert_eq!(Rssi::from_str("0").unwrap().dbm(), 0);
    }

    #[test]
    fn from_str_invalid() {
        assert!(Rssi::from_str("abc").is_err());
        assert!(Rssi::from_str("").is_err());
    }
}
