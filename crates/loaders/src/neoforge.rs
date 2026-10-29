//! NeoForge loader — the NeoForged maven versions API.

use std::path::{Path, PathBuf};

use serde::Deserialize;
use simple_download_utility::DownloadProgress;
use tokio::sync::mpsc::Sender;

use crate::LoaderKind;
use crate::error::LoaderError;
use crate::http::{download_files, get_json};
use crate::meta::{LoaderVersion, ModInfo};
use crate::mod_info;
use crate::sealed;

/// Marker for the NeoForge loader (https://neoforged.net).
pub struct NeoForge;

const VERSIONS_URL: &str = "https://maven.neoforged.net/api/maven/versions/releases/net/neoforged/neoforge";
const MAVEN_BASE: &str = "https://maven.neoforged.net/releases/net/neoforged/neoforge";

/// Maps a Minecraft version to the NeoForge version prefix that targets it.
///
/// NeoForge encodes the Minecraft version into its own: `1.20.4` → `20.4.x`,
/// `1.21` → `21.0.x`, `1.21.1` → `21.1.x`, snapshot `25w14craftmine` →
/// `0.25w14craftmine.x`, and post-`1.x` numeric Minecraft versions (`26.1`)
/// map directly (`26.1.x`). Snapshots are letter-carrying (`25w14craftmine`
/// starts with digits but is not fully numeric), which is how they are told
/// apart from the post-`1.x` era.
fn minecraft_prefix(minecraft: &str) -> String {
    if let Some(rest) = minecraft.strip_prefix("1.") {
        if rest.split('.').count() == 1 {
            format!("{rest}.0.")
        } else {
            format!("{rest}.")
        }
    } else if minecraft.chars().all(|c| c.is_ascii_digit() || c == '.') {
        format!("{minecraft}.")
    } else {
        format!("0.{minecraft}.")
    }
}

/// Pre-release suffixes the NeoForged maven uses; anything else is stable.
fn is_stable(version: &str) -> bool {
    !(version.ends_with("-beta") || version.ends_with("-prerelease") || version.ends_with("-rc"))
}

/// The versions API's envelope: `{ "isSnapshot": bool, "versions": [...] }`.
#[derive(Debug, Deserialize)]
struct NeoForgeVersions {
    #[serde(default)]
    #[allow(dead_code)] // part of the API's shape; surfaced as a whole via `versions`
    is_snapshot: bool,
    versions: Vec<String>,
}

async fn fetch_versions(http: &reqwest::Client) -> Result<Vec<String>, LoaderError> {
    let json = get_json(http, VERSIONS_URL).await?;
    serde_json::from_value::<NeoForgeVersions>(json)
        .map(|versions| versions.versions)
        .map_err(|source| LoaderError::Json {
            url: VERSIONS_URL.to_string(),
            source,
        })
}

fn artifact_urls(version: &str) -> [(String, PathBuf); 2] {
    let installer = format!("neoforge-{version}-installer.jar");
    let universal = format!("neoforge-{version}-universal.jar");
    [
        (format!("{MAVEN_BASE}/{version}/{installer}"), PathBuf::from(installer)),
        (format!("{MAVEN_BASE}/{version}/{universal}"), PathBuf::from(universal)),
    ]
}

impl sealed::Sealed for NeoForge {}
impl LoaderKind for NeoForge {
    const NAME: &'static str = "neoforge";
    const SUPPORTS_MOD_INFO: bool = true;

    async fn list_versions(http: &reqwest::Client) -> Result<Vec<LoaderVersion>, LoaderError> {
        let versions = fetch_versions(http).await?;
        Ok(versions
            .iter()
            .map(|version| LoaderVersion::new(version.clone(), None, is_stable(version)))
            .collect())
    }

    async fn versions_for_minecraft(http: &reqwest::Client, minecraft: &str) -> Result<Vec<LoaderVersion>, LoaderError> {
        let prefix = minecraft_prefix(minecraft);
        let versions = fetch_versions(http).await?;
        Ok(versions
            .iter()
            .filter(|version| version.starts_with(&prefix))
            .map(|version| LoaderVersion::new(version.clone(), Some(minecraft.to_string()), is_stable(version)))
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
#[path = "../tests/neoforge.rs"]
mod tests;
