use std::fmt::Display;
use std::fmt::Formatter;
use std::str::FromStr;
use std::sync::Arc;

use serde::Deserialize;
use serde::Serialize;
use thiserror::Error;

use crate::BtleplugTransport;
use crate::ble::transport::BleTransport;
use crate::ble::virt::VirtualClockTransport;
use crate::error::Result;

/// BLE backend selection for transport creation.
///
/// Allows runtime switching between real BLE hardware and a virtual
/// in-memory device for testing.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum Backend {
    /// Real BLE hardware via `btleplug`.
    #[default]
    Btleplug,
    /// Virtual in-memory device for testing.
    Virtual,
}

/// Error parsing a [`Backend`] from a string.
#[derive(Debug, Clone, PartialEq, Eq, Error)]
#[error("invalid backend '{input}': {reason}")]
pub struct BackendParseError {
    /// The raw input string.
    pub input: String,
    /// The parse error reason.
    pub reason: String,
}

impl Display for Backend {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Btleplug => write!(f, "btleplug"),
            Self::Virtual => write!(f, "virtual"),
        }
    }
}

impl FromStr for Backend {
    type Err = BackendParseError;

    fn from_str(s: &str) -> std::result::Result<Self, Self::Err> {
        match s.to_ascii_lowercase().as_str() {
            "btleplug" | "ble" | "real" => Ok(Self::Btleplug),
            "virtual" | "mock" => Ok(Self::Virtual),
            _ => Err(BackendParseError {
                input: s.to_string(),
                reason: "expected 'btleplug' or 'virtual'".to_string(),
            }),
        }
    }
}

impl Backend {
    /// Create a BLE transport for this backend.
    ///
    /// `Btleplug` creates a real BLE transport. `Virtual` creates a virtual
    /// in-memory device pre-loaded with a scan advertisement for the given MAC
    /// (if provided), so `scan` returns a result immediately.
    pub async fn create_transport(self) -> Result<Arc<dyn BleTransport>> {
        match self {
            Self::Btleplug => {
                let transport = BtleplugTransport::new().await?;
                Ok(Arc::new(transport))
            }
            Self::Virtual => Ok(Arc::new(VirtualClockTransport::new())),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn backend_default_is_btleplug() {
        assert_eq!(Backend::default(), Backend::Btleplug);
    }

    #[test]
    fn backend_from_str_btleplug() {
        assert_eq!("btleplug".parse::<Backend>().unwrap(), Backend::Btleplug);
        assert_eq!("ble".parse::<Backend>().unwrap(), Backend::Btleplug);
        assert_eq!("real".parse::<Backend>().unwrap(), Backend::Btleplug);
    }

    #[test]
    fn backend_from_str_virtual() {
        assert_eq!("virtual".parse::<Backend>().unwrap(), Backend::Virtual);
        assert_eq!("mock".parse::<Backend>().unwrap(), Backend::Virtual);
    }

    #[test]
    fn backend_from_str_invalid() {
        let err = "foo".parse::<Backend>().unwrap_err();
        assert_eq!(err.input, "foo");
        assert!(err.reason.contains("expected"));
    }

    #[test]
    fn backend_display() {
        assert_eq!(Backend::Btleplug.to_string(), "btleplug");
        assert_eq!(Backend::Virtual.to_string(), "virtual");
    }

    #[tokio::test]
    async fn create_transport_virtual() {
        let transport = Backend::Virtual.create_transport().await.unwrap();
        assert!(!transport.is_connected());
    }
}
