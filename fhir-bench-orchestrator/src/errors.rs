//! This module simplifies and standardizes error handling for the project.

/// The standard `Result<T>` alias to use throughout the application.
pub type Result<T> = ::std::result::Result<T, AppError>;

/// The standard error type to use throughout the application. This is the error type used by the
/// application's `Result<T>` alias. For every error type encountered in the application, an entry
/// should be added to this enum, in addition to a `From` impl to convert from the raw error to
/// the enum entry.
#[derive(Debug)]
pub enum AppError {
    SetLoggerError(log::SetLoggerError),
    VarError(std::env::VarError),
    IoError(std::io::Error),
    ParseIntError(std::num::ParseIntError),

    /// Represents an error caused by a child process exiting abnormally.
    ChildProcessFailure(std::process::ExitStatus, String),

    /// Represents an error caused by an attempt to lookup an unknown server.
    UnknownServerError(&'static crate::servers::ServerName),
}

impl From<log::SetLoggerError> for AppError {
    fn from(err: log::SetLoggerError) -> AppError {
        AppError::SetLoggerError(err)
    }
}

impl From<std::env::VarError> for AppError {
    fn from(err: std::env::VarError) -> AppError {
        AppError::VarError(err)
    }
}

impl From<std::io::Error> for AppError {
    fn from(err: std::io::Error) -> AppError {
        AppError::IoError(err)
    }
}

impl From<std::num::ParseIntError> for AppError {
    fn from(err: std::num::ParseIntError) -> AppError {
        AppError::ParseIntError(err)
    }
}
