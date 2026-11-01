//! The errors a write or a read can fail with.

use std::io;

/// Why an export or a read failed.
#[derive(Debug, thiserror::Error)]
pub enum Error {
    /// A file could not be read or written.
    #[error("file error: {0}")]
    Io(#[from] io::Error),
    /// A zip archive could not be written.
    #[error("zip error: {0}")]
    Zip(#[from] zip::result::ZipError),
    /// A manifest could not be encoded or decoded.
    #[error("manifest error: {0}")]
    Json(#[from] serde_json::Error),
    /// The write was cancelled before it finished.
    #[error("the export was cancelled")]
    Cancelled,
    /// The package is not one this reader understands.
    #[error("{0}")]
    Invalid(String),
}

/// The result of an export or a read.
pub type Result<T> = std::result::Result<T, Error>;
