use crate::error::Result;
use crate::token::AuthToken;
use crate::types::MacAddress;

/// Storage backend for auth tokens.
///
/// A token is only persisted after a privileged command (e.g., time sync)
/// succeeds. An Auth Confirm ACK alone does not prove the token was accepted.
pub trait TokenStore: Send + Sync {
    /// Load the token for a device address.
    fn load(&self, address: &MacAddress) -> Option<AuthToken>;

    /// Save the token for a device address.
    fn save(&self, address: &MacAddress, token: &AuthToken) -> Result<()>;
}
