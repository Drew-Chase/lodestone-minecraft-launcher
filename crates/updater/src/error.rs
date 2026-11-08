//! What the update check can fail on.

use std::path::{Path, PathBuf};

/// A failure in the update check, the download, or the install.
#[derive(Debug, thiserror::Error)]
pub enum Error {
    /// GitHub could not be reached, or answered with an error.
    #[error("could not reach GitHub: {0}")]
    Request(#[from] reqwest::Error),
    /// A tag or a version is not `X.Y.Z`, with an optional leading `v` on a tag.
    #[error("{0:?} is not a version")]
    Version(String),
    /// The release does not carry a file that the update needs.
    #[error("the release is missing {0}")]
    MissingAsset(String),
    /// A file or folder could not be written.
    #[error("could not write {}: {source}", .path.display())]
    Io {
        path: PathBuf,
        #[source]
        source: std::io::Error,
    },
    /// The downloaded file does not match the release's checksum. It has been deleted.
    #[error("the download does not match its checksum")]
    Checksum,
    /// The launcher could not start the installer, or start itself again after an update.
    #[error("{0}")]
    Launch(String),
    /// The archive could not be unpacked, or it does not hold the file the update needs.
    #[error("{0}")]
    Unpack(String),
    /// The release's checksums are not signed by the release key, so nothing is downloaded from it.
    #[error("{0}")]
    Signature(String),
}

/// An `Io` error for `path`.
pub(crate) fn io_error(path: &Path, source: std::io::Error) -> Error {
    Error::Io {
        path: path.to_path_buf(),
        source,
    }
}
