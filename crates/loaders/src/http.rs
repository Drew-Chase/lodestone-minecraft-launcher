//! Shared HTTP and download helpers (private).

use std::path::{Path, PathBuf};

use reqwest::Client;
use simple_download_utility::{DownloadProgress, download_file};
use tokio::sync::mpsc::Sender;

use crate::error::LoaderError;

/// GETs `url` and deserializes the JSON body, mapping failures onto
/// [`LoaderError`] with the url attached.
pub(crate) async fn get_json(http: &Client, url: &str) -> Result<serde_json::Value, LoaderError> {
    let response = http.get(url).send().await.map_err(|source| LoaderError::Network {
        url: url.to_string(),
        source,
    })?;
    let status = response.status();
    if !status.is_success() {
        return Err(LoaderError::UnexpectedResponse {
            url: url.to_string(),
            status: status.as_u16(),
        });
    }
    // Read the body as text and parse from the string so parse failures surface
    // as real `serde_json::Error`s rather than reqwest's opaque wrapper.
    let body = response.text().await.map_err(|source| LoaderError::Network {
        url: url.to_string(),
        source,
    })?;
    serde_json::from_str(&body).map_err(|source| LoaderError::Json {
        url: url.to_string(),
        source,
    })
}

/// How many files a loader download fetches at once.
const PARALLEL: usize = 64;

/// Downloads `(url, destination)` pairs concurrently, creating parent directories as
/// needed, and returns the destination paths in the order given. A file already on disk
/// is kept, so a repeat install downloads nothing it already has.
///
/// A progress channel, when given, receives per-byte progress for each file —
/// the same `DownloadProgress` events piston-mc's downloader emits, so gui can
/// treat every download uniformly.
pub(crate) async fn download_files(
    files: impl IntoIterator<Item = (String, PathBuf)>,
    progress: Option<&Sender<DownloadProgress>>,
) -> Result<Vec<PathBuf>, LoaderError> {
    use futures_util::{StreamExt, TryStreamExt};

    let progress = progress.cloned();
    futures_util::stream::iter(files)
        .map(move |(url, path)| {
            let progress = progress.clone();
            async move {
                fetch(&url, &path, progress).await?;
                Ok::<PathBuf, LoaderError>(path)
            }
        })
        .buffered(PARALLEL)
        .try_collect::<Vec<PathBuf>>()
        .await
}

/// Fetches one file into `path` unless a non-empty copy is already there. The download
/// lands in a `.part` file that is renamed once complete, so an interrupted download is
/// never mistaken for a finished one.
async fn fetch(url: &str, path: &Path, progress: Option<Sender<DownloadProgress>>) -> Result<(), LoaderError> {
    if std::fs::metadata(path).is_ok_and(|meta| meta.is_file() && meta.len() > 0) {
        return Ok(());
    }
    if let Some(parent) = path.parent() {
        std::fs::create_dir_all(parent).map_err(|source| LoaderError::Io {
            path: parent.to_path_buf(),
            source,
        })?;
    }
    let mut partial = path.as_os_str().to_owned();
    partial.push(".part");
    let partial = PathBuf::from(partial);
    download_file(url, &partial, progress).await.map_err(|source| LoaderError::Download {
        url: url.to_string(),
        source: source.into(),
    })?;
    std::fs::rename(&partial, path).map_err(|source| LoaderError::Io {
        path: path.to_path_buf(),
        source,
    })
}

/// Turns a maven coordinate `"group:artifact:version[:classifier]"` into its
/// repository-relative jar path
/// `group/artifact/version/artifact-version[-classifier].jar`.
pub(crate) fn maven_jar_path(coordinate: &str) -> Option<String> {
    let mut parts = coordinate.split(':');
    let group = parts.next()?;
    let artifact = parts.next()?;
    let version = parts.next()?;
    let classifier = parts.next();
    let file = match classifier {
        Some(classifier) if !classifier.is_empty() => format!("{artifact}-{version}-{classifier}.jar"),
        _ => format!("{artifact}-{version}.jar"),
    };
    Some(format!("{}/{}/{}/{}", group.replace('.', "/"), artifact, version, file))
}

/// Ensures a maven base url ends with exactly one slash so artifact paths can be
/// appended verbatim.
pub(crate) fn maven_base(url: &str) -> String {
    let trimmed = url.trim_end_matches('/');
    format!("{trimmed}/")
}

#[cfg(test)]
#[path = "../tests/http.rs"]
mod tests;
