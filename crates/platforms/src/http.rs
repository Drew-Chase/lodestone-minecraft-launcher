//! Shared HTTP and download helpers (private).
//!
//! Deliberately closure-free at the call sites of trait methods: closures
//! capturing the surrounding method's reference arguments trip rustc's RPITIT
//! `+ Send` check ("implementation of FnOnce is not general enough") — see
//! `lodestone_loaders`.

use std::path::PathBuf;

use reqwest::Client;
use simple_download_utility::{DownloadProgress, download_file};
use tokio::sync::mpsc::Sender;

use crate::error::PlatformError;

/// GETs `url` and deserializes the JSON body, mapping failures onto
/// [`PlatformError`] with the url attached.
pub(crate) async fn get_json(http: &Client, url: &str) -> Result<serde_json::Value, PlatformError> {
    let response = http.get(url).send().await.map_err(|source| PlatformError::Network {
        url: url.to_string(),
        source,
    })?;
    let status = response.status();
    if !status.is_success() {
        return Err(PlatformError::UnexpectedResponse {
            url: url.to_string(),
            status: status.as_u16(),
        });
    }
    // Read the body as text and parse from the string so parse failures surface
    // as real `serde_json::Error`s rather than reqwest's opaque wrapper.
    let body = response.text().await.map_err(|source| PlatformError::Network {
        url: url.to_string(),
        source,
    })?;
    serde_json::from_str(&body).map_err(|source| PlatformError::Json {
        url: url.to_string(),
        source,
    })
}

/// Downloads `(url, destination)` pairs sequentially, creating parent
/// directories as needed, and returns the destination paths in order.
///
/// A progress channel, when given, receives per-byte progress for each file —
/// the same `DownloadProgress` events piston-mc and lodestone_loaders emit, so
/// gui can treat every download uniformly.
pub(crate) async fn download_files(
    files: impl IntoIterator<Item = (String, PathBuf)>,
    progress: Option<&Sender<DownloadProgress>>,
) -> Result<Vec<PathBuf>, PlatformError> {
    let mut downloaded = Vec::new();
    for (url, path) in files {
        if let Some(parent) = path.parent() {
            std::fs::create_dir_all(parent).map_err(|source| PlatformError::Io {
                path: parent.to_path_buf(),
                source,
            })?;
        }
        download_file(&url, &path, progress.cloned())
            .await
            .map_err(|source| PlatformError::Download {
                url: url.clone(),
                source: source.into(),
            })?;
        downloaded.push(path);
    }
    Ok(downloaded)
}

/// Appends a query parameter to `url` with proper percent-encoding.
pub(crate) fn with_query(url: &str, key: &str, value: &str) -> String {
    let separator = if url.contains('?') { '&' } else { '?' };
    format!("{url}{separator}{key}={}", percent_encode(value))
}

/// Percent-encodes everything outside the unreserved set — enough for query
/// values like search terms.
fn percent_encode(value: &str) -> String {
    let mut encoded = String::with_capacity(value.len());
    for byte in value.bytes() {
        match byte {
            b'A'..=b'Z' | b'a'..=b'z' | b'0'..=b'9' | b'-' | b'_' | b'.' | b'~' => {
                encoded.push(byte as char);
            }
            _ => {
                encoded.push('%');
                encoded.push_str(&format!("{byte:02X}"));
            }
        }
    }
    encoded
}

#[cfg(test)]
#[path = "../tests/http.rs"]
mod tests;
