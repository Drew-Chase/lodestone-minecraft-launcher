//! The errors snapshot files can fail with.

use std::path::PathBuf;

/// What a snapshot read, write or restore can fail with.
#[derive(Debug, thiserror::Error)]
pub enum Error {
    /// A file could not be read or written.
    #[error("{0}")]
    Io(#[from] std::io::Error),
    /// A snapshot's JSON could not be written or read.
    #[error("{0}")]
    Json(#[from] serde_json::Error),
    /// A zip of local jars could not be written or read.
    #[error("{0}")]
    Zip(#[from] zip::result::ZipError),
    /// A jar taken from a snapshot does not have the SHA-1 the snapshot recorded for it.
    #[error("{file} does not match the SHA-1 its snapshot recorded")]
    Checksum { file: String },
    /// A file a restore would write is already in place.
    #[error("{} already exists", .0.display())]
    Exists(PathBuf),
}

/// The result of a snapshot operation.
pub type Result<T> = std::result::Result<T, Error>;
