//! Forge loader — `files.minecraftforge.net`'s maven-metadata.json.

use std::collections::BTreeMap;
use std::path::{Path, PathBuf};

use simple_download_utility::DownloadProgress;
use tokio::sync::mpsc::Sender;

use crate::LoaderKind;
use crate::error::LoaderError;
use crate::http::{download_files, get_json};
use crate::meta::{LoaderVersion, ModInfo};
use crate::mod_info;
use crate::sealed;

/// Marker for the MinecraftForge loader (https://files.minecraftforge.net).
pub struct Forge;

const METADATA_URL: &str = "https://files.minecraftforge.net/net/minecraftforge/forge/maven-metadata.json";
const MAVEN_BASE: &str = "https://maven.minecraftforge.net/net/minecraftforge/forge";

/// The Minecraft version embedded at the front of a combined forge version:
/// `"1.21.1-52.1.20"` → `"1.21.1"`, `"1.7.10-10.13.4.1614-1.7.10"` → `"1.7.10"`
/// (the split is on the *first* dash, so suffixed builds stay intact).
fn minecraft_from_combined(version: &str) -> Option<String> {
    version.split_once('-').map(|(minecraft, _)| minecraft.to_string())
}

async fn fetch_index(http: &reqwest::Client) -> Result<BTreeMap<String, Vec<String>>, LoaderError> {
    let json = get_json(http, METADATA_URL).await?;
    serde_json::from_value(json).map_err(|source| LoaderError::Json {
        url: METADATA_URL.to_string(),
        source,
    })
}

/// The maven artifacts that exist for every forge build, across all eras. The
/// `client.jar` is produced by the installer at install time and is not
/// published, so it is not fetched here.
fn artifact_urls(version: &str) -> [(String, PathBuf); 2] {
    let installer = format!("forge-{version}-installer.jar");
    let universal = format!("forge-{version}-universal.jar");
    [
        (format!("{MAVEN_BASE}/{version}/{installer}"), PathBuf::from(installer)),
        (format!("{MAVEN_BASE}/{version}/{universal}"), PathBuf::from(universal)),
    ]
}

impl sealed::Sealed for Forge {}
impl LoaderKind for Forge {
    const NAME: &'static str = "forge";
    const SUPPORTS_MOD_INFO: bool = true;

    async fn list_versions(http: &reqwest::Client) -> Result<Vec<LoaderVersion>, LoaderError> {
        let index = fetch_index(http).await?;
        Ok(index
            .into_values()
            .flatten()
            .map(|version| LoaderVersion::new(version.clone(), minecraft_from_combined(&version), true))
            .collect())
    }

    async fn versions_for_minecraft(http: &reqwest::Client, minecraft: &str) -> Result<Vec<LoaderVersion>, LoaderError> {
        let index = fetch_index(http).await?;
        Ok(index
            .get(minecraft)
            .into_iter()
            .flatten()
            .map(|version| LoaderVersion::new(version.clone(), Some(minecraft.to_string()), true))
            .collect())
    }

    async fn download_client(
        _http: &reqwest::Client,
        version: &LoaderVersion,
        into: &Path,
        progress: Option<&Sender<DownloadProgress>>,
    ) -> Result<Vec<PathBuf>, LoaderError> {
        if version.minecraft.is_none() {
            return Err(LoaderError::MinecraftVersionUnknown {
                version: version.version.clone(),
            });
        }
        let files = artifact_urls(&version.version).map(|(url, name)| (url, into.join(name)));
        download_files(files, progress).await
    }

    fn extract_mod_info(jar: &Path) -> Result<ModInfo, LoaderError> {
        mod_info::extract(jar)
    }
}

#[cfg(test)]
#[path = "../tests/forge.rs"]
mod tests;
