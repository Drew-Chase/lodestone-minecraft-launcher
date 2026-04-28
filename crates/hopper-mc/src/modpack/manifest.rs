use std::collections::HashMap;

use serde::{Deserialize, Serialize};

/// Which platform the modpack archive originated from.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum ModpackSource {
    Modrinth,
    CurseForge,
}

/// Environment filter for a file within a modpack.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum ModpackFileEnv {
    Client,
    Server,
    Both,
}

/// A single file entry inside a modpack archive.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ModpackFile {
    /// Relative path within the instance (e.g. `"mods/sodium-0.5.8.jar"`).
    pub path: String,
    /// Direct download URLs. Empty for CurseForge packs until resolved.
    pub download_urls: Vec<String>,
    /// File size in bytes, if known.
    pub size: Option<u64>,
    /// Hash algorithm → hex digest (e.g. `sha1`, `sha512`).
    pub hashes: HashMap<String, String>,
    /// Whether this file is required.
    pub required: bool,
    /// Client/server environment filter.
    pub env: Option<ModpackFileEnv>,
    /// CurseForge project ID (for file resolution).
    pub project_id: Option<String>,
    /// CurseForge file ID (for file resolution).
    pub file_id: Option<String>,
}

/// Parsed modpack manifest, unified across Modrinth and CurseForge formats.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ModpackManifest {
    pub name: String,
    pub version: Option<String>,
    pub author: Option<String>,
    pub minecraft_version: String,
    /// Mod loader name: `"fabric"`, `"forge"`, `"neoforge"`, `"quilt"`.
    pub loader: String,
    pub loader_version: String,
    pub files: Vec<ModpackFile>,
    pub source: ModpackSource,
}
