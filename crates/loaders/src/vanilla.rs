//! Vanilla Minecraft: Mojang's version list and the client jar each version names, all fetched through the launcher's
//! network settings and checked against the hashes Mojang publishes.

use std::path::{Path, PathBuf};

use lodestone_net::download::{Target, sha1_of_file};
use piston_mc::manifest_v2::{ManifestV2, ReleaseType};
use simple_download_utility::DownloadProgress;
use tokio::sync::mpsc::Sender;

use crate::LoaderKind;
use crate::error::LoaderError;
use crate::install::InstalledVersion;
use crate::meta::{LoaderVersion, ModInfo};
use crate::mojang;
use crate::sealed;

/// Marker for vanilla Minecraft. Its "loader versions" are Minecraft versions
/// themselves, sourced from Mojang's piston-meta manifest.
pub struct Vanilla;

/// A failed request to Mojang, with the address that failed.
fn fetch_error(url: &str, source: reqwest::Error) -> LoaderError {
    LoaderError::Download {
        url: url.to_string(),
        source: source.into(),
    }
}

async fn version_list(http: &reqwest::Client) -> Result<ManifestV2, LoaderError> {
    mojang::version_list(http)
        .await
        .map_err(|source| fetch_error(mojang::VERSION_LIST_URL, source))
}

fn manifest_version(manifest_version: &piston_mc::manifest_v2::Version) -> LoaderVersion {
    LoaderVersion::new(
        manifest_version.id.clone(),
        Some(manifest_version.id.clone()),
        manifest_version.release_type == ReleaseType::Release,
    )
}

impl sealed::Sealed for Vanilla {}
impl LoaderKind for Vanilla {
    const NAME: &'static str = "vanilla";
    const SUPPORTS_MOD_INFO: bool = false;

    async fn list_versions(http: &reqwest::Client) -> Result<Vec<LoaderVersion>, LoaderError> {
        let manifest = version_list(http).await?;
        Ok(manifest.versions.iter().map(manifest_version).collect())
    }

    async fn versions_for_minecraft(http: &reqwest::Client, minecraft: &str) -> Result<Vec<LoaderVersion>, LoaderError> {
        let manifest = version_list(http).await?;
        Ok(manifest
            .versions
            .iter()
            .filter(|version| version.id == minecraft)
            .map(manifest_version)
            .collect())
    }

    async fn download_client(
        http: &reqwest::Client,
        version: &LoaderVersion,
        into: &Path,
        progress: Option<&Sender<DownloadProgress>>,
    ) -> Result<Vec<PathBuf>, LoaderError> {
        let manifest = version_list(http).await?;
        let entry = manifest
            .versions
            .iter()
            .find(|entry| entry.id == version.version)
            .ok_or_else(|| LoaderError::VersionNotFound {
                version: version.version.clone(),
                minecraft: version.version.clone(),
            })?;
        let version_json = mojang::version_json(http, &entry.url)
            .await
            .map_err(|source| fetch_error(&entry.url, source))?;
        let client = version_json.downloads.client;

        std::fs::create_dir_all(into).map_err(|source| LoaderError::Io {
            path: into.to_path_buf(),
            source,
        })?;
        let destination = into.join(format!("{}.jar", version.version));
        let target = Target {
            url: client.url.clone(),
            path: destination.clone(),
            sha1: Some(client.sha1.clone()),
            size: Some(client.size),
        };
        if already_there(destination.clone(), client.sha1, client.size).await {
            return Ok(vec![destination]);
        }
        lodestone_net::download::fetch(http, &target, |received, total| {
            if let Some(progress) = progress {
                let _ = progress.try_send(DownloadProgress {
                    bytes_to_download: total.unwrap_or(received) as usize,
                    bytes_downloaded: received as usize,
                    bytes_per_second: 0,
                });
            }
        })
        .await
        .map_err(|source| LoaderError::Download {
            url: target.url.clone(),
            source: source.into(),
        })?;
        Ok(vec![destination])
    }

    async fn install(
        _http: &reqwest::Client,
        _version: &LoaderVersion,
        _game_dir: &Path,
        _java: &Path,
        _lines: Option<&Sender<String>>,
        _progress: Option<&Sender<DownloadProgress>>,
    ) -> Result<InstalledVersion, LoaderError> {
        Err(LoaderError::InstallUnsupported { loader: Self::NAME })
    }

    fn extract_mod_info(_jar: &Path) -> Result<ModInfo, LoaderError> {
        Err(LoaderError::ModInfoUnsupported { loader: Self::NAME })
    }
}

/// Whether the file at `path` is already the `size` bytes and SHA-1 that Mojang names, so it need not be downloaded again.
async fn already_there(path: PathBuf, sha1: String, size: u64) -> bool {
    tokio::task::spawn_blocking(move || {
        std::fs::metadata(&path).is_ok_and(|meta| meta.is_file() && meta.len() == size)
            && sha1_of_file(&path).is_ok_and(|actual| actual.eq_ignore_ascii_case(&sha1))
    })
    .await
    .unwrap_or(false)
}
