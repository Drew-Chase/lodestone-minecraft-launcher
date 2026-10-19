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
mod tests {
    use super::*;

    const SAMPLE: &str = r#"{
        "manifestType": "minecraftModpack",
        "manifestVersion": 1,
        "name": "Test Pack",
        "version": "1.2.3",
        "author": "Alice",
        "minecraft": {
            "version": "1.20.1",
            "modLoaders": [
                { "id": "forge-47.4.0", "primary": true }
            ]
        },
        "files": [
            { "projectID": 306612, "fileID": 4838285, "required": true },
            { "projectID": 238222, "fileID": 4655861, "required": false }
        ],
        "overrides": "overrides"
    }"#;

    #[test]
    fn the_manifest_maps_onto_modpack() {
        let modpack = parse(SAMPLE, Path::new("test.zip")).expect("sample parses");
        assert_eq!(modpack.name, "Test Pack");
        assert_eq!(modpack.version.as_deref(), Some("1.2.3"));
        assert_eq!(modpack.minecraft, "1.20.1");
        assert_eq!(modpack.loader, Some("forge"));
        assert_eq!(modpack.loader_version.as_deref(), Some("47.4.0"));
        assert_eq!(modpack.overrides.as_deref(), Some("overrides"));
        assert_eq!(modpack.files.len(), 2);
        assert_eq!(modpack.files[0].project_id.as_deref(), Some("306612"));
        assert_eq!(modpack.files[0].file_id.as_deref(), Some("4838285"));
        assert!(modpack.files[0].url.is_empty(), "urls need the CurseForge API at install time");
    }

    #[test]
    fn loader_ids_split_on_the_first_dash() {
        assert_eq!(split_loader("forge-47.4.0"), (Some("forge"), Some("47.4.0".to_string())));
        assert_eq!(split_loader("neoforge-21.1.95"), (Some("neoforge"), Some("21.1.95".to_string())));
        assert_eq!(split_loader("mystery-1.0"), (None, None));
        assert_eq!(split_loader("bare"), (None, None));
    }

    #[test]
    fn a_non_primary_loader_is_used_when_no_primary_exists() {
        let document = r#"{
            "minecraft": { "version": "1.20.1", "modLoaders": [ { "id": "fabric-0.16.14" } ] }
        }"#;
        let modpack = parse(document, Path::new("test.zip")).expect("parses");
        assert_eq!(modpack.loader, Some("fabric"));
        assert_eq!(modpack.loader_version.as_deref(), Some("0.16.14"));
    }
}
