use crate::CharacteristicUuid;
use crate::MacAddress;
use thiserror::Error;

/// BLE transport error details.
///
/// Replaces the former `Transport(String)` variant with structured
/// error information for each known transport failure mode.
#[derive(Debug, Clone, PartialEq, Eq, Error)]
pub enum TransportError {
    /// No Bluetooth adapter was found on the system.
    #[error("no Bluetooth adapter found")]
    NoAdapter,

    /// The device with the given address was not found during scanning.
    #[error("device not found: {address}")]
    DeviceNotFound {
        /// The MAC address that was searched for.
        address: MacAddress,
    },

    /// The requested GATT characteristic was not found on the device.
    #[error("characteristic not found: {characteristic}")]
    CharacteristicNotFound {
        /// The characteristic that was requested.
        characteristic: CharacteristicUuid,
    },

    /// No read value was queued for the requested characteristic.
    #[error("no read value set for characteristic")]
    NoReadValue,

    /// A pending request was canceled before a response arrived.
    #[error("pending request canceled")]
    RequestCanceled,

    /// A response was canceled before it could be delivered.
    #[error("{context} response canceled")]
    ResponseCanceled {
        /// What kind of response was canceled (e.g. "firmware", "settings").
        context: String,
    },

    /// Reconnect with state recovery failed.
    #[error("reconnect with state recovery failed")]
    ReconnectFailed,

    /// The device is not connected.
    #[error("not connected")]
    NotConnected,

    /// The connected device MAC is unknown to the transport.
    #[error("unknown device MAC: {mac}")]
    UnknownDeviceMac {
        /// The MAC address that was not recognized.
        mac: MacAddress,
    },

    /// The transport does not support reading the given characteristic.
    #[error("transport does not support read for characteristic {characteristic}")]
    UnsupportedRead {
        /// The characteristic that was requested.
        characteristic: CharacteristicUuid,
    },

    /// System time error (e.g. clock went backwards).
    #[error("system time error: {0}")]
    SystemTime(String),

    /// Catch-all for other transport errors with a custom message.
    #[error("{0}")]
    Other(String),
}
