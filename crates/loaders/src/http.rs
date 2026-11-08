//! Shared HTTP and download helpers (private).

use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicUsize, Ordering};

use lodestone_net::download::Target;
use reqwest::Client;
use simple_download_utility::DownloadProgress;
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
static CONCURRENCY: AtomicUsize = AtomicUsize::new(64);

/// Sets how many files a loader download fetches at once. The Settings page sets it.
pub fn set_concurrency(count: usize) {
    CONCURRENCY.store(count, Ordering::Relaxed);
}

/// Downloads `files` concurrently with `http`, creating parent directories as needed, and returns the destination paths
/// in the order given. A file already on disk is kept when it matches the size and SHA-1 its target names (or is
/// non-empty, when it names neither), so a repeat install downloads nothing it already has. Every download must answer
/// with a success and match its checks before it lands (see [`lodestone_net::download`]).
///
/// A progress channel, when given, receives per-byte progress for each file —
/// the same `DownloadProgress` events piston-mc's downloader emits, so gui can
/// treat every download uniformly.
pub(crate) async fn download_files(
    http: &Client,
    files: Vec<Target>,
    progress: Option<&Sender<DownloadProgress>>,
) -> Result<Vec<PathBuf>, LoaderError> {
    use futures_util::{StreamExt, TryStreamExt};

    let progress = progress.cloned();
    futures_util::stream::iter(files)
        .map(move |target| {
            let progress = progress.clone();
            async move {
                fetch(http, &target, progress).await?;
                Ok::<PathBuf, LoaderError>(target.path)
            }
        })
        .buffered(CONCURRENCY.load(Ordering::Relaxed).max(1))
        .try_collect::<Vec<PathBuf>>()
        .await
}

/// Fetches one target unless a copy that passes its checks is already there.
async fn fetch(http: &Client, target: &Target, progress: Option<Sender<DownloadProgress>>) -> Result<(), LoaderError> {
    if is_present(target).await {
        return Ok(());
    }
    lodestone_net::download::fetch(http, target, |received, total| {
        if let Some(progress) = &progress {
            let _ = progress.try_send(DownloadProgress {
                bytes_to_download: total.unwrap_or(received) as usize,
                bytes_downloaded: received as usize,
                bytes_per_second: 0,
            });
        }
    })
    .await
    .map(drop)
    .map_err(|source| LoaderError::Download {
        url: target.url.clone(),
        source: source.into(),
    })
}

/// Whether the file at `target.path` is already the one `target` names: at its size, and hashing to its SHA-1 when it
/// names one. A target that names neither only has to be a non-empty file.
async fn is_present(target: &Target) -> bool {
    let (path, sha1, size) = (target.path.clone(), target.sha1.clone(), target.size);
    tokio::task::spawn_blocking(move || {
        let Ok(meta) = std::fs::metadata(&path) else {
            return false;
        };
        if !meta.is_file() || meta.len() == 0 || size.is_some_and(|size| size != meta.len()) {
            return false;
        }
        sha1.is_none_or(|expected| sha1_of(&path).is_some_and(|actual| actual.eq_ignore_ascii_case(&expected)))
    })
    .await
    .unwrap_or(false)
}

/// The SHA-1 of the file at `path`, as lowercase hex.
fn sha1_of(path: &Path) -> Option<String> {
    use sha1::{Digest, Sha1};
    let mut file = std::fs::File::open(path).ok()?;
    let mut hasher = Sha1::new();
    std::io::copy(&mut file, &mut hasher).ok()?;
    Some(hasher.finalize().iter().map(|byte| format!("{byte:02x}")).collect())
}

/// The SHA-1 a maven repository publishes beside `url`, at `<url>.sha1`, when it publishes one.
pub(crate) async fn maven_sha1(http: &Client, url: &str) -> Option<String> {
    let response = http.get(format!("{url}.sha1")).send().await.ok()?.error_for_status().ok()?;
    let text = response.text().await.ok()?;
    let hash = text.split_whitespace().next()?;
    (hash.len() == 40 && hash.bytes().all(|byte| byte.is_ascii_hexdigit())).then(|| hash.to_ascii_lowercase())
}

/// Downloads a Forge-style loader's installer and, when the maven has one, its universal jar into `into`, checked
/// against the SHA-1 the maven publishes beside each. The installer is required; a universal jar the maven does not
/// have (an answer of 404) is skipped, since only the installer runs.
pub(crate) async fn download_installer(
    http: &Client,
    installer: (String, PathBuf),
    universal: (String, PathBuf),
    progress: Option<&Sender<DownloadProgress>>,
) -> Result<Vec<PathBuf>, LoaderError> {
    let installer = Target {
        sha1: maven_sha1(http, &installer.0).await,
        ..Target::unchecked(installer.0, installer.1)
    };
    let mut paths = download_files(http, vec![installer], progress).await?;
    let universal = Target {
        sha1: maven_sha1(http, &universal.0).await,
        ..Target::unchecked(universal.0, universal.1)
    };
    match download_files(http, vec![universal], progress).await {
        Ok(more) => paths.extend(more),
        Err(LoaderError::Download { url, source }) if is_not_found(source.as_ref()) => {
            tracing::debug!("{url} is not published, so the installer runs without it");
        }
        Err(error) => return Err(error),
    }
    Ok(paths)
}

/// Whether a download failed because the server has no such file.
fn is_not_found(error: &(dyn std::error::Error + 'static)) -> bool {
    matches!(
        error.downcast_ref::<lodestone_net::download::Error>(),
        Some(lodestone_net::download::Error::Status { status: 404, .. })
    )
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
