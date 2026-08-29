/// Library-level result alias.
pub type Result<T> = std::result::Result<T, ClockError>;

/// Errors returned by the cgd1-rs library.
#[derive(Debug, thiserror::Error)]
pub enum ClockError {
    /// BLE transport error.
    #[error("BLE transport error: {0}")]
    Transport(String),

    /// Authentication failed (token rejected by device).
    #[error("authentication failed: {0}")]
    AuthFailed(String),

    /// No authentication token available for this device.
    #[error("no auth token: device not paired")]
    NoAuthToken,

    /// Command was rejected by the device (non-success ACK status).
    #[error("command rejected: command={command:#04x}, status={status}")]
    CommandRejected {
        /// The command byte that was rejected.
        command: u8,
        /// The ACK status from the device.
        status: crate::command::AckStatus,
    },

    /// Timeout waiting for a response from the device.
    #[error("timeout waiting for response")]
    Timeout,

    /// Device is not connected.
    #[error("not connected")]
    NotConnected,

    /// Device is already connected.
    #[error("already connected")]
    AlreadyConnected,

    /// Invalid alarm slot index (must be 0-15).
    #[error("invalid alarm slot: {0}")]
    InvalidAlarmSlot(u8),

    /// Invalid settings value (out of range).
    #[error("invalid settings value: {0}")]
    InvalidSettings(String),

    /// Failed to parse an advertisement or notification.
    #[error("parse error: {0}")]
    Parse(String),

    /// I/O error.
    #[error("IO error: {0}")]
    Io(#[from] std::io::Error),

    /// Internal btleplug error.
    #[error("btleplug error: {0}")]
    Btleplug(#[from] btleplug::Error),
}
