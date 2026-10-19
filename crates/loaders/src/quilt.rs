//! Quilt loader — `meta.quiltmc.org` v3, which shares fabric's entry schema.

use std::path::{Path, PathBuf};

use simple_download_utility::DownloadProgress;
use tokio::sync::mpsc::Sender;

use crate::LoaderKind;
use crate::error::LoaderError;
use crate::fabric;
use crate::meta::{LoaderVersion, ModInfo};
use crate::mod_info;
use crate::sealed;

/// Marker for the Quilt loader (https://quiltmc.org).
pub struct Quilt;

const META_BASE: &str = "https://meta.quiltmc.org/v3/versions/loader";
const DEFAULT_MAVEN: &str = "https://maven.quiltmc.org/repository/release/";

impl sealed::Sealed for Quilt {}
impl LoaderKind for Quilt {
    const NAME: &'static str = "quilt";
    const SUPPORTS_MOD_INFO: bool = true;

    async fn list_versions(http: &reqwest::Client) -> Result<Vec<LoaderVersion>, LoaderError> {
        let entries = fabric::list(http, META_BASE).await?;
        Ok(entries.iter().map(|entry| fabric::entry_version(entry, None)).collect())
    }

    async fn versions_for_minecraft(http: &reqwest::Client, minecraft: &str) -> Result<Vec<LoaderVersion>, LoaderError> {
        let url = format!("{META_BASE}/{minecraft}");
        let entries = fabric::list(http, &url).await?;
        Ok(entries.iter().map(|entry| fabric::entry_version(entry, Some(minecraft))).collect())
    }

    async fn download_client(
        http: &reqwest::Client,
        version: &LoaderVersion,
        into: &Path,
        progress: Option<&Sender<DownloadProgress>>,
    ) -> Result<Vec<PathBuf>, LoaderError> {
        let Some(minecraft) = version.minecraft.as_deref() else {
            return Err(LoaderError::MinecraftVersionUnknown {
                version: version.version.clone(),
            });
        };
        let url = format!("{META_BASE}/{minecraft}");
        let entries = fabric::list(http, &url).await?;
        let mut found = None;
        for entry in &entries {
            if entry.loader.version == version.version {
                found = Some(entry);
                break;
            }
        }
        let entry = match found {
            Some(entry) => entry,
            None => {
                return Err(LoaderError::VersionNotFound {
                    version: version.version.clone(),
                    minecraft: minecraft.to_string(),
                });
            }
        };
        fabric::download_libraries(entry, DEFAULT_MAVEN, into, progress).await
    }

    /// Quilt metadata is parsed locally (modcrawl has no `quilt.mod.json`
    /// support yet). Jars without it but with `fabric.mod.json` are quilt's own
    /// fabric-compatibility case, so the fabric path is used instead — and the
    /// returned `ModInfo.loader` honestly reports `"fabric"`.
    fn extract_mod_info(jar: &Path) -> Result<ModInfo, LoaderError> {
        match mod_info::quilt::extract(jar) {
            Ok(info) => Ok(info),
            Err(LoaderError::ModMetadataMissing { .. }) => mod_info::extract(jar),
            Err(error) => Err(error),
        }
    }
}
