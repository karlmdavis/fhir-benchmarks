//! This module contains the application's fatal error types.

use thiserror::Error;

/// Enumerates the application's custom unrecoverable errors. For every unrecoverable error type
/// encountered in the application, an entry should be added to this enum. Note: the
/// [thiserror](https://github.com/dtolnay/thiserror) library is used to derive the error details.
#[derive(Debug, Error)]
pub enum AppError {
    /// Represents an error caused by a child process exiting abnormally.
    #[error("child process exited with code '{0}' and this message: '{1}'")]
    ChildProcessFailure(std::process::ExitStatus, String),

    /// Represents an error caused by an attempt to lookup an unknown server.
    #[error("unknown server '{0}'")]
    UnknownServerError(crate::servers::ServerName),
}