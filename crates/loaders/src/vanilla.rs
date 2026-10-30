//! Vanilla Minecraft — delegated to piston-mc, the workspace's Mojang API layer.

use std::path::{Path, PathBuf};

use anyhow::Error as AnyhowError;
use piston_mc::manifest_v2::{ManifestV2, ReleaseType};
use simple_download_utility::DownloadProgress;
use tokio::sync::mpsc::Sender;

use crate::LoaderKind;
use crate::error::LoaderError;
use crate::install::InstalledVersion;
use crate::meta::{LoaderVersion, ModInfo};
use crate::sealed;

/// Marker for vanilla Minecraft. Its "loader versions" are Minecraft versions
/// themselves, sourced from Mojang's piston-meta manifest.
pub struct Vanilla;

/// piston-mc reports failures as `anyhow` errors without structured context; the
/// concrete network/download work still shows up in the source chain.
fn vanilla_error(source: AnyhowError) -> LoaderError {
    LoaderError::Download {
        url: "Mojang piston-meta (via piston-mc)".to_string(),
        source: source.into(),
    }
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

    async fn list_versions(_http: &reqwest::Client) -> Result<Vec<LoaderVersion>, LoaderError> {
        let manifest = ManifestV2::fetch().await.map_err(vanilla_error)?;
        Ok(manifest.versions.iter().map(manifest_version).collect())
    }

    async fn versions_for_minecraft(_http: &reqwest::Client, minecraft: &str) -> Result<Vec<LoaderVersion>, LoaderError> {
        let manifest = ManifestV2::fetch().await.map_err(vanilla_error)?;
        Ok(manifest
            .versions
            .iter()
            .filter(|version| version.id == minecraft)
            .map(manifest_version)
            .collect())
    }

    async fn download_client(
        _http: &reqwest::Client,
        version: &LoaderVersion,
        into: &Path,
        progress: Option<&Sender<DownloadProgress>>,
    ) -> Result<Vec<PathBuf>, LoaderError> {
        let manifest = ManifestV2::fetch().await.map_err(vanilla_error)?;
        let version_manifest = manifest
            .version(&version.version)
            .await
            .map_err(vanilla_error)?
            .ok_or_else(|| LoaderError::VersionNotFound {
                version: version.version.clone(),
                minecraft: version.version.clone(),
            })?;

        std::fs::create_dir_all(into).map_err(|source| LoaderError::Io {
            path: into.to_path_buf(),
            source,
        })?;
        let destination = into.join(format!("{}.jar", version.version));
        version_manifest
            .download_client(&destination, true, progress.cloned())
            .await
            .map_err(vanilla_error)?;
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
