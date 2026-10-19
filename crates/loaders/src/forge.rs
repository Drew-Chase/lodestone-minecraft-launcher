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
mod tests {
    use super::*;

    /// Recorded slice of the real `maven-metadata.json`: the map is keyed by
    /// Minecraft version, and the values are the full combined version strings
    /// (including odd suffixes like `1.7.10-10.13.4.1614-1.7.10`).
    const SAMPLE_INDEX: &str = r#"{
        "1.1": ["1.1-1.3.2.1", "1.1-1.3.4.29"],
        "1.7.10": ["1.7.10-10.13.0.1150", "1.7.10-10.13.4.1614-1.7.10"],
        "1.21.1": ["1.21.1-52.0.0", "1.21.1-52.1.20"]
    }"#;

    #[test]
    fn the_index_parses_into_a_sorted_map() {
        let index: BTreeMap<String, Vec<String>> = serde_json::from_str(SAMPLE_INDEX).expect("real metadata shape");
        let keys: Vec<&str> = index.keys().map(String::as_str).collect();
        assert_eq!(keys, ["1.1", "1.21.1", "1.7.10"]);
        assert_eq!(index["1.21.1"].len(), 2);
    }

    #[test]
    fn combined_versions_embed_their_minecraft_version() {
        assert_eq!(minecraft_from_combined("1.21.1-52.1.20").as_deref(), Some("1.21.1"));
        assert_eq!(minecraft_from_combined("1.7.10-10.13.4.1614-1.7.10").as_deref(), Some("1.7.10"));
        assert_eq!(minecraft_from_combined("nonsense"), None);
    }

    #[test]
    fn artifact_urls_cover_installer_and_universal() {
        let [(installer_url, installer_path), (universal_url, universal_path)] = artifact_urls("1.7.10-10.13.4.1614-1.7.10");
        assert_eq!(
            installer_url,
            "https://maven.minecraftforge.net/net/minecraftforge/forge/1.7.10-10.13.4.1614-1.7.10/forge-1.7.10-10.13.4.1614-1.7.10-installer.jar"
        );
        assert_eq!(installer_path, PathBuf::from("forge-1.7.10-10.13.4.1614-1.7.10-installer.jar"));
        assert_eq!(
            universal_url,
            "https://maven.minecraftforge.net/net/minecraftforge/forge/1.7.10-10.13.4.1614-1.7.10/forge-1.7.10-10.13.4.1614-1.7.10-universal.jar"
        );
        assert_eq!(universal_path, PathBuf::from("forge-1.7.10-10.13.4.1614-1.7.10-universal.jar"));
    }

    #[test]
    fn versions_for_an_unknown_minecraft_version_are_empty() {
        let index: BTreeMap<String, Vec<String>> = serde_json::from_str(SAMPLE_INDEX).expect("real metadata shape");
        assert!(!index.contains_key("2.0.0"));
    }
}
