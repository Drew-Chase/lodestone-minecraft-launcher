use anyhow::Result;
use serde::{Deserialize, Serialize};

const API_URL: &str = "https://meta.fabricmc.net/v2/versions/";

/// Response from the Fabric meta API containing all version information.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FabricVersions {
    pub game: Vec<GameVersion>,
    pub loader: Vec<LoaderVersion>,
    pub intermediary: Vec<IntermediaryVersion>,
    pub installer: Vec<InstallerVersion>,
}

/// A Minecraft game version supported by Fabric.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GameVersion {
    pub version: String,
    pub stable: bool,
}

/// A Fabric loader version.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LoaderVersion {
    pub separator: String,
    pub build: u32,
    pub maven: String,
    pub version: String,
    pub stable: bool,
}

/// An intermediary mappings version.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct IntermediaryVersion {
    pub maven: String,
    pub version: String,
    pub stable: bool,
}

/// A Fabric installer version.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct InstallerVersion {
    pub url: String,
    pub maven: String,
    pub version: String,
    pub stable: bool,
}

impl FabricVersions {
    pub async fn fetch() -> Result<Self> {
        let response = reqwest::get(API_URL).await?;
        let versions = response.json::<Self>().await?;
        Ok(versions)
    }
}
