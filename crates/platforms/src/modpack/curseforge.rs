//! CurseForge export parsing — `.zip` archives with a `manifest.json`
//! (https://www.curseforge.com; export format documented community-wide).
//!
//! Manifest files reference CurseForge ids (`projectID`/`fileID`) rather than
//! urls; resolving those ids is the install slice's job via the CurseForge
//! kind's API calls.

use std::path::Path;

use serde::Deserialize;

use crate::error::PlatformError;
use crate::meta::{Modpack, ModpackFile};

/// The `manifest.json` of a CurseForge export.
#[derive(Debug, Deserialize)]
struct Manifest {
    #[serde(default)]
    name: Option<String>,
    #[serde(default)]
    version: Option<String>,
    #[serde(default)]
    minecraft: Option<ManifestMinecraft>,
    #[serde(default)]
    files: Vec<ManifestFile>,
    #[serde(default)]
    overrides: Option<String>,
}

#[derive(Debug, Deserialize)]
struct ManifestMinecraft {
    #[serde(default)]
    version: Option<String>,
    #[serde(rename = "modLoaders", default)]
    mod_loaders: Vec<ManifestModLoader>,
}

#[derive(Debug, Deserialize)]
struct ManifestModLoader {
    /// `"forge-47.2.0"`, `"fabric-0.16.14"`, `"neoforge-21.1.95"`, …
    id: String,
    #[serde(default)]
    primary: bool,
}

#[derive(Debug, Deserialize)]
struct ManifestFile {
    #[serde(rename = "projectID")]
    project_id: i64,
    #[serde(rename = "fileID")]
    file_id: i64,
}

/// Splits a `modLoaders[].id` into its loader name and version.
fn split_loader(id: &str) -> (Option<&'static str>, Option<String>) {
    let Some((name, version)) = id.split_once('-') else {
        return (None, None);
    };
    let loader = match name {
        "forge" => "forge",
        "fabric" => "fabric",
        "quilt" => "quilt",
        "neoforge" => "neoforge",
        _ => return (None, None),
    };
    (Some(loader), Some(version.to_string()))
}

pub(super) fn parse(raw: &str, archive: &Path) -> Result<Modpack, PlatformError> {
    let manifest: Manifest = serde_json::from_str(raw).map_err(|source| PlatformError::ModpackParse {
        path: archive.to_path_buf(),
        source: Box::new(source),
    })?;

    // The primary loader wins; manifests typically carry exactly one.
    let (loader, loader_version) = manifest
        .minecraft
        .as_ref()
        .map(|minecraft| {
            let primary = minecraft.mod_loaders.iter().find(|loader| loader.primary);
            let loader = primary.or_else(|| minecraft.mod_loaders.first());
            loader.map(|loader| split_loader(&loader.id)).unwrap_or((None, None))
        })
        .unwrap_or((None, None));

    let minecraft = manifest.minecraft.and_then(|minecraft| minecraft.version).unwrap_or_default();

    Ok(Modpack {
        name: manifest.name.unwrap_or_else(|| "CurseForge modpack".to_string()),
        version: manifest.version,
        minecraft,
        loader,
        loader_version,
        files: manifest
            .files
            .into_iter()
            .map(|file| ModpackFile {
                path: String::new(),
                url: String::new(),
                sha1: None,
                size: None,
                project_id: Some(file.project_id.to_string()),
                file_id: Some(file.file_id.to_string()),
            })
            .collect(),
        overrides: manifest.overrides,
    })
}

#[cfg(test)]
#[path = "../../tests/modpack/curseforge.rs"]
mod tests;
