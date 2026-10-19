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
mod tests {
    use super::*;

    const SAMPLE: &str = r#"{
        "formatVersion": 1,
        "game": "minecraft",
        "versionId": "1.2.3",
        "name": "Test Pack",
        "summary": "A test pack",
        "dependencies": {
            "minecraft": "1.21.1",
            "fabric-loader": "0.16.14"
        },
        "files": [
            {
                "path": "mods/sodium.jar",
                "hashes": { "sha1": "abc", "sha512": "def" },
                "env": { "client": "required", "server": "required" },
                "downloads": ["https://cdn.modrinth.com/data/x/versions/y/sodium.jar"],
                "fileSize": 1234
            },
            {
                "path": "mods/no-url.jar",
                "hashes": { "sha1": "zzz" },
                "downloads": []
            }
        ],
        "overrides": "overrides"
    }"#;

    #[test]
    fn the_mrpack_manifest_maps_onto_modpack() {
        let modpack = parse(SAMPLE, Path::new("test.mrpack")).expect("sample parses");
        assert_eq!(modpack.name, "Test Pack");
        assert_eq!(modpack.version.as_deref(), Some("1.2.3"));
        assert_eq!(modpack.minecraft, "1.21.1");
        assert_eq!(modpack.loader, Some("fabric"));
        assert_eq!(modpack.loader_version.as_deref(), Some("0.16.14"));
        assert_eq!(modpack.overrides.as_deref(), Some("overrides"));
        // The file without a mirror is dropped; it cannot be installed.
        assert_eq!(modpack.files.len(), 1);
        assert_eq!(modpack.files[0].path, "mods/sodium.jar");
        assert_eq!(modpack.files[0].sha1.as_deref(), Some("abc"));
        assert_eq!(modpack.files[0].size, Some(1234));
    }

    #[test]
    fn every_loader_dependency_key_maps() {
        for (key, expected) in [("forge", "forge"), ("quilt-loader", "quilt"), ("neoforge", "neoforge")] {
            let document = format!(r#"{{ "dependencies": {{ "minecraft": "1.0", "{key}": "9.9" }} }}"#);
            let index: MrpackIndex = serde_json::from_str(&document).expect("parses");
            let (loader, version) = index.dependencies.loader();
            assert_eq!(loader, Some(expected));
            assert_eq!(version.as_deref(), Some("9.9"));
        }
    }

    #[test]
    fn a_vanilla_pack_has_no_loader() {
        let index: MrpackIndex = serde_json::from_str(r#"{ "dependencies": { "minecraft": "1.21.1" } }"#).expect("parses");
        assert_eq!(index.dependencies.loader(), (None, None));
    }
}
