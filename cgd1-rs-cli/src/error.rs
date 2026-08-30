use cgd1_rs::AuthFailedError;
use cgd1_rs::ClockError;
use std::path::PathBuf;
use thiserror::Error;

/// Errors returned by the CLI.
#[derive(Debug, miette::Diagnostic, Error)]
pub enum CliError {
    /// A core library error.
    #[error(transparent)]
    #[diagnostic(code(cgd1_cli::core))]
    Core(#[from] ClockError),

    /// Authentication failed with context about the token.
    #[error(transparent)]
    #[diagnostic(code(cgd1_cli::auth_failed))]
    AuthFailed(#[from] AuthFailedError),

    /// Audio file could not be read.
    #[error("failed to read audio file '{}': {reason}", path.display())]
    #[diagnostic(code(cgd1_cli::audio_read_failed))]
    AudioReadFailed {
        /// The file path.
        path: PathBuf,
        /// The I/O error reason.
        reason: String,
    },
}
