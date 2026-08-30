mod auth;
mod clock;
mod transport;

pub use auth::AuthFailedError;
pub use clock::ClockError;
pub use transport::TransportError;

/// Library-level result alias.
pub type Result<T> = std::result::Result<T, ClockError>;
