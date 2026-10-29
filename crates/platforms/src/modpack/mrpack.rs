//! `.mrpack` parsing — Modrinth's modpack format
//! (https://docs.modrinth.com/docs/modpacks/format_definition/).

use std::path::Path;

use serde::Deserialize;

use crate::error::PlatformError;
use crate::meta::{Modpack, ModpackFile};

/// The `modrinth.index.json` manifest.
#[derive(Debug, Deserialize)]
struct MrpackIndex {
    #[serde(default)]
    name: Option<String>,
    #[serde(rename = "versionId", default)]
    version_id: Option<String>,
    #[serde(default)]
    dependencies: MrpackDependencies,
    #[serde(default)]
    files: Vec<MrpackFile>,
    #[serde(default)]
    overrides: Option<String>,
}

/// Loader + Minecraft versions the modpack targets. The dependency keys use
/// hyphens (`fabric-loader`, `quilt-loader`) per the mrpack spec.
#[derive(Debug, Default, Deserialize)]
struct MrpackDependencies {
    #[serde(default)]
    minecraft: Option<String>,
    #[serde(rename = "fabric-loader", default)]
    fabric_loader: Option<String>,
    #[serde(default)]
    forge: Option<String>,
    #[serde(rename = "quilt-loader", default)]
    quilt_loader: Option<String>,
    #[serde(default)]
    neoforge: Option<String>,
}

impl MrpackDependencies {
    /// The loader name and version, derived from which dependency key is set.
    fn loader(&self) -> (Option<&'static str>, Option<String>) {
        if let Some(version) = &self.fabric_loader {
            (Some("fabric"), Some(version.clone()))
        } else if let Some(version) = &self.forge {
            (Some("forge"), Some(version.clone()))
        } else if let Some(version) = &self.quilt_loader {
            (Some("quilt"), Some(version.clone()))
        } else if let Some(version) = &self.neoforge {
            (Some("neoforge"), Some(version.clone()))
        } else {
            (None, None)
        }
    }
}

/// One file the modpack installs: a direct download plus its destination path.
#[derive(Debug, Deserialize)]
struct MrpackFile {
    path: String,
    /// The `hashes` map carries `sha1` and `sha512`.
    #[serde(rename = "hashes", default)]
    hash: Option<MrpackHashes>,
    /// Mirrored download urls; the first is used.
    downloads: Vec<String>,
    #[serde(rename = "fileSize", default)]
    file_size: Option<u64>,
}

#[derive(Debug, Deserialize)]
struct MrpackHashes {
    #[serde(default)]
    sha1: Option<String>,
}

pub(super) fn parse(raw: &str, archive: &Path) -> Result<Modpack, PlatformError> {
    let index: MrpackIndex = serde_json::from_str(raw).map_err(|source| PlatformError::ModpackParse {
        path: archive.to_path_buf(),
        source: Box::new(source),
    })?;

    let (loader, loader_version) = index.dependencies.loader();
    let files = index
        .files
        .into_iter()
        .filter_map(|file| {
            // Mirrors are optional; a file without any url cannot be installed.
            let url = file.downloads.into_iter().next()?;
            Some(ModpackFile {
                path: file.path,
                url,
                sha1: file.hash.and_then(|hash| hash.sha1),
                size: file.file_size,
                project_id: None,
                file_id: None,
            })
        })
        .collect();

    Ok(Modpack {
        name: index.name.unwrap_or_else(|| "Modrinth modpack".to_string()),
        version: index.version_id,
        minecraft: index.dependencies.minecraft.unwrap_or_default(),
        loader,
        loader_version,
        files,
        overrides: index.overrides,
    })
}

#[cfg(test)]
#[path = "../../tests/modpack/mrpack.rs"]
mod tests;
