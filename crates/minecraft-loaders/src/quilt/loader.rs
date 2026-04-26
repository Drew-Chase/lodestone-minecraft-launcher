use anyhow::{anyhow, Result};
use serde::{Deserialize, Serialize};
use std::path::{Path, PathBuf};
use tokio::fs;
use tokio::io::AsyncWriteExt;

const GAME_VERSIONS_URL: &str = "https://meta.quiltmc.org/v3/versions/game";
const LOADER_VERSIONS_URL: &str = "https://meta.quiltmc.org/v3/versions/loader";
const INSTALLER_VERSIONS_URL: &str = "https://meta.quiltmc.org/v3/versions/installer";

/// All available Quilt versions, fetched from the Quilt Meta API.
/// The API is structurally similar to Fabric's.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct QuiltVersions {
    pub game: Vec<GameVersion>,
    pub loader: Vec<LoaderVersion>,
    pub installer: Vec<InstallerVersion>,
}

/// A Minecraft game version supported by Quilt.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GameVersion {
    pub version: String,
    pub stable: bool,
}

/// A Quilt loader version.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LoaderVersion {
    pub version: String,
    #[serde(default)]
    pub maven: String,
    #[serde(default)]
    pub build: u32,
}

/// A Quilt installer version.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct InstallerVersion {
    pub version: String,
    #[serde(default)]
    pub url: String,
    #[serde(default)]
    pub maven: String,
}

impl QuiltVersions {
    /// Fetch all available Quilt versions from the Meta API.
    /// Makes three parallel requests for game, loader, and installer versions.
    pub async fn fetch() -> Result<Self> {
        let game_res = reqwest::get(GAME_VERSIONS_URL).await?;
        let loader_res = reqwest::get(LOADER_VERSIONS_URL).await?;
        let installer_res = reqwest::get(INSTALLER_VERSIONS_URL).await?;

        let game = game_res.json::<Vec<GameVersion>>().await?;
        let loader = loader_res.json::<Vec<LoaderVersion>>().await?;
        let installer = installer_res.json::<Vec<InstallerVersion>>().await?;

        Ok(Self {
            game,
            loader,
            installer,
        })
    }

    /// Gets the latest loader version (first in list, which is newest).
    pub fn get_latest_loader(&self) -> Option<&LoaderVersion> {
        self.loader.first()
    }

    /// Gets the latest installer version.
    pub fn get_latest_installer(&self) -> Option<&InstallerVersion> {
        self.installer.first()
    }

    /// Gets the latest stable game version.
    pub fn get_latest_game_version(&self) -> Option<&GameVersion> {
        self.game.iter().find(|v| v.stable)
    }

    /// Finds a specific loader version by version string.
    pub fn find_loader(&self, version: &str) -> Option<&LoaderVersion> {
        self.loader.iter().find(|v| v.version == version)
    }

    /// Finds a specific game version by version string.
    pub fn find_game_version(&self, version: &str) -> Option<&GameVersion> {
        self.game.iter().find(|v| v.version == version)
    }

    /// Checks whether a specific Minecraft version is supported by Quilt.
    pub fn supports_game_version(&self, version: &str) -> bool {
        self.game.iter().any(|v| v.version == version)
    }
}

impl InstallerVersion {
    /// Downloads the installer JAR to a specified path.
    pub async fn download(&self, output_path: impl AsRef<Path>) -> Result<PathBuf> {
        if self.url.is_empty() {
            return Err(anyhow!("No download URL available for installer {}", self.version));
        }
        let response = reqwest::get(&self.url).await?;
        let bytes = response.bytes().await?;
        let output = output_path.as_ref();

        if let Some(parent) = output.parent() {
            fs::create_dir_all(parent).await?;
        }

        let mut file = fs::File::create(output).await?;
        file.write_all(&bytes).await?;

        Ok(output.to_path_buf())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_fetch_versions() {
        let versions = QuiltVersions::fetch().await;
        assert!(versions.is_ok());
        let versions = versions.unwrap();
        assert!(!versions.game.is_empty());
        assert!(!versions.loader.is_empty());
        assert!(!versions.installer.is_empty());
    }

    #[tokio::test]
    async fn test_get_latest_versions() {
        let versions = QuiltVersions::fetch().await.unwrap();
        assert!(versions.get_latest_loader().is_some());
        assert!(versions.get_latest_installer().is_some());
        assert!(versions.get_latest_game_version().is_some());
    }

    #[tokio::test]
    async fn test_supports_game_version() {
        let versions = QuiltVersions::fetch().await.unwrap();
        // Quilt should support at least some recent MC version
        let latest = versions.get_latest_game_version().unwrap();
        assert!(versions.supports_game_version(&latest.version));
        assert!(!versions.supports_game_version("0.0.1"));
    }
}
