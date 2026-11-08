//! Downloading one file so that a broken download never looks finished.
//!
//! The answer must be a success, the bytes stream into a part file beside the target, the SHA-1 and the size are
//! checked while they stream, and only a file that passed both is renamed into place. A failed or abandoned download
//! removes its part file, so the target is either the old file, the new file, or absent.

use std::fmt;
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicU64, Ordering};

use sha1::{Digest, Sha1};
use tokio::io::AsyncWriteExt;

/// One file to download: where from, where to, and what it must match. A `None` check is skipped.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Target {
    pub url: String,
    pub path: PathBuf,
    /// The SHA-1 the file must hash to, as hex in either case.
    pub sha1: Option<String>,
    /// The size in bytes the file must have.
    pub size: Option<u64>,
}

impl Target {
    /// A target with no hash or size to check.
    pub fn unchecked(url: impl Into<String>, path: impl Into<PathBuf>) -> Self {
        Self {
            url: url.into(),
            path: path.into(),
            sha1: None,
            size: None,
        }
    }
}

/// Why a download failed.
#[derive(Debug)]
pub enum Error {
    /// The request could not be sent, or the body stopped arriving.
    Request { url: String, source: reqwest::Error },
    /// The server answered with something other than a success.
    Status { url: String, status: u16 },
    /// The part file or the target could not be written.
    Io { path: PathBuf, source: std::io::Error },
    /// The body was not the size the target expects.
    Size { url: String, expected: u64, received: u64 },
    /// The body did not hash to the SHA-1 the target expects.
    Sha1 { url: String, expected: String, received: String },
}

impl fmt::Display for Error {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Error::Request { url, source } => write!(f, "could not download {url}: {source}"),
            Error::Status { url, status } => write!(f, "{url} answered with HTTP status {status}"),
            Error::Io { path, source } => write!(f, "could not write {}: {source}", path.display()),
            Error::Size { url, expected, received } => {
                write!(f, "{url} sent {received} bytes, but {expected} were expected")
            }
            Error::Sha1 { url, expected, received } => {
                write!(f, "{url} does not match its SHA-1 (expected {expected}, got {received})")
            }
        }
    }
}

impl std::error::Error for Error {
    fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
        match self {
            Error::Request { source, .. } => Some(source),
            Error::Io { source, .. } => Some(source),
            _ => None,
        }
    }
}

/// Downloads `target` with `client`, and returns the number of bytes written. `progress` is called after each chunk with
/// the bytes received so far and the total, when the target or the server names one. Every byte also counts towards
/// [`crate::bytes_received`].
pub async fn fetch(client: &reqwest::Client, target: &Target, mut progress: impl FnMut(u64, Option<u64>)) -> Result<u64, Error> {
    let url = target.url.as_str();
    let request = |source| Error::Request {
        url: url.to_string(),
        source,
    };
    let mut response = client.get(url).send().await.map_err(request)?;
    let status = response.status();
    if !status.is_success() {
        return Err(Error::Status {
            url: url.to_string(),
            status: status.as_u16(),
        });
    }
    if let (Some(expected), Some(announced)) = (target.size, response.content_length())
        && expected != announced
    {
        return Err(Error::Size {
            url: url.to_string(),
            expected,
            received: announced,
        });
    }
    let total = target.size.or_else(|| response.content_length());

    if let Some(parent) = target.path.parent() {
        tokio::fs::create_dir_all(parent).await.map_err(|source| Error::Io {
            path: parent.to_path_buf(),
            source,
        })?;
    }
    let part = PartFile::beside(&target.path);
    let io = |source| Error::Io {
        path: part.path.clone(),
        source,
    };
    let mut file = tokio::fs::File::create(&part.path).await.map_err(io)?;
    let mut hasher = target.sha1.as_ref().map(|_| Sha1::new());
    let mut received: u64 = 0;
    while let Some(chunk) = response.chunk().await.map_err(request)? {
        received += chunk.len() as u64;
        if let Some(expected) = target.size
            && received > expected
        {
            return Err(Error::Size {
                url: url.to_string(),
                expected,
                received,
            });
        }
        if let Some(hasher) = hasher.as_mut() {
            hasher.update(&chunk);
        }
        file.write_all(&chunk).await.map_err(io)?;
        crate::record_bytes(chunk.len() as u64);
        progress(received, total);
    }
    file.flush().await.map_err(io)?;
    file.sync_all().await.map_err(io)?;
    drop(file);

    if let Some(expected) = target.size
        && received != expected
    {
        return Err(Error::Size {
            url: url.to_string(),
            expected,
            received,
        });
    }
    if let (Some(expected), Some(hasher)) = (target.sha1.as_deref(), hasher) {
        let digest = hex(&hasher.finalize());
        if !digest.eq_ignore_ascii_case(expected) {
            return Err(Error::Sha1 {
                url: url.to_string(),
                expected: expected.to_string(),
                received: digest,
            });
        }
    }
    part.finish(&target.path).await?;
    Ok(received)
}

/// The SHA-1 of the file at `path`, as lowercase hex.
pub fn sha1_of_file(path: &Path) -> std::io::Result<String> {
    let mut file = std::fs::File::open(path)?;
    let mut hasher = Sha1::new();
    std::io::copy(&mut file, &mut hasher)?;
    Ok(hex(&hasher.finalize()))
}

/// The SHA-1 of `bytes`, as lowercase hex.
pub fn sha1_hex(bytes: &[u8]) -> String {
    hex(&Sha1::digest(bytes))
}

/// Lowercase hex of `bytes`.
fn hex(bytes: &[u8]) -> String {
    use std::fmt::Write;
    bytes.iter().fold(String::with_capacity(bytes.len() * 2), |mut out, byte| {
        let _ = write!(out, "{byte:02x}");
        out
    })
}

/// A part file beside a target. Each download gets its own name, so two downloads of one target never write the same
/// part file. It is removed when dropped unless it was renamed into place.
struct PartFile {
    path: PathBuf,
    finished: bool,
}

impl PartFile {
    fn beside(target: &Path) -> Self {
        static NEXT: AtomicU64 = AtomicU64::new(0);
        let serial = NEXT.fetch_add(1, Ordering::Relaxed);
        let name = target.file_name().map_or_else(|| "download".into(), |name| name.to_string_lossy());
        let path = target.with_file_name(format!(".{name}.{}-{serial}.part", std::process::id()));
        Self { path, finished: false }
    }

    /// Renames the part file over `target`.
    async fn finish(mut self, target: &Path) -> Result<(), Error> {
        tokio::fs::rename(&self.path, target).await.map_err(|source| Error::Io {
            path: target.to_path_buf(),
            source,
        })?;
        self.finished = true;
        Ok(())
    }
}

impl Drop for PartFile {
    fn drop(&mut self) {
        if !self.finished {
            let _ = std::fs::remove_file(&self.path);
        }
    }
}

#[cfg(test)]
#[path = "../tests/download.rs"]
mod tests;
