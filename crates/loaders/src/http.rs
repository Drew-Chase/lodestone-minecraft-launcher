//! Shared HTTP and download helpers (private).

use std::path::PathBuf;

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

/// Downloads `(url, destination)` pairs sequentially, creating parent directories
/// as needed, and returns the destination paths in order.
///
/// A progress channel, when given, receives per-byte progress for each file —
/// the same `DownloadProgress` events piston-mc's downloader emits, so gui can
/// treat every download uniformly.
pub(crate) async fn download_files(
    files: impl IntoIterator<Item = (String, PathBuf)>,
    progress: Option<&Sender<DownloadProgress>>,
) -> Result<Vec<PathBuf>, LoaderError> {
    let mut downloaded = Vec::new();
    for (url, path) in files {
        if let Some(parent) = path.parent() {
            std::fs::create_dir_all(parent).map_err(|source| LoaderError::Io {
                path: parent.to_path_buf(),
                source,
            })?;
        }
        download_file(&url, &path, progress.cloned())
            .await
            .map_err(|source| LoaderError::Download {
                url: url.clone(),
                source: source.into(),
            })?;
        downloaded.push(path);
    }
    Ok(downloaded)
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
mod tests {
    use super::*;

    #[test]
    fn maven_coordinates_map_to_repository_paths() {
        assert_eq!(
            maven_jar_path("net.fabricmc:fabric-loader:0.16.14").as_deref(),
            Some("net/fabricmc/fabric-loader/0.16.14/fabric-loader-0.16.14.jar")
        );
        assert_eq!(
            maven_jar_path("net.fabricmc:sponge-mixin:0.12.5+mixin.0.8.5").as_deref(),
            Some("net/fabricmc/sponge-mixin/0.12.5+mixin.0.8.5/sponge-mixin-0.12.5+mixin.0.8.5.jar")
        );
        assert_eq!(
            maven_jar_path("org.ow2.asm:asm:9.7.1:sysv").as_deref(),
            Some("org/ow2/asm/asm/9.7.1/asm-9.7.1-sysv.jar")
        );
        assert_eq!(maven_jar_path("not-a-coordinate"), None);
    }

    #[test]
    fn maven_bases_keep_a_single_trailing_slash() {
        assert_eq!(maven_base("https://maven.fabricmc.net/"), "https://maven.fabricmc.net/");
        assert_eq!(maven_base("https://maven.fabricmc.net"), "https://maven.fabricmc.net/");
    }
}
