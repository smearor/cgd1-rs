use std::ops::Deref;

use crate::token::AuthToken;

/// Result of loading or generating an auth token.
///
/// Indicates whether the token was loaded from storage (`is_new = false`)
/// or freshly generated (`is_new = true`).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct TokenResult {
    /// The authentication token.
    token: AuthToken,
    /// Whether the token was newly generated (not found in storage).
    is_new: bool,
}

impl TokenResult {
    /// Create a result wrapping an existing (loaded) token.
    pub(crate) fn existing(token: AuthToken) -> Self {
        Self { token, is_new: false }
    }

    /// Create a result wrapping a freshly generated token.
    pub(crate) fn generated(token: AuthToken) -> Self {
        Self { token, is_new: true }
    }

    /// Whether the token was newly generated (not found in storage).
    pub fn is_new(&self) -> bool {
        self.is_new
    }
}

impl Deref for TokenResult {
    type Target = AuthToken;

    fn deref(&self) -> &Self::Target {
        &self.token
    }
}
