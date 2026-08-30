use thiserror::Error;

/// Authentication failure details.
///
/// Carries the original device-side reason and optional context about
/// whether the token was newly generated or loaded from storage.
#[derive(Debug, Clone, PartialEq, Eq, Error)]
#[error("{reason}")]
pub struct AuthFailedError {
    /// The original reason from the device (e.g. status code).
    pub reason: String,
    /// Whether the token was newly generated (not found in storage).
    pub is_new_token: bool,
    /// Filesystem path to the token file, if applicable.
    pub token_path: Option<String>,
}
