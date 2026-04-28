use std::collections::HashMap;
use std::io::{Read, Seek};

use serde::Deserialize;

use super::manifest::{ModpackFile, ModpackFileEnv, ModpackManifest, ModpackSource};
use crate::error::ContentError;

// ---------------------------------------------------------------------------
// Modrinth index DTOs
// ---------------------------------------------------------------------------

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
struct MrIndex {
    #[allow(dead_code)]
    format_version: u32,
    #[allow(dead_code)]
    game: String,
    version_id: String,
    name: String,
    #[allow(dead_code)]
    summary: Option<String>,
    files: Vec<MrFile>,
    dependencies: HashMap<String, String>,
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
struct MrFile {
    path: String,
    hashes: HashMap<String, String>,
    env: Option<MrEnv>,
    downloads: Vec<String>,
    file_size: Option<u64>,
}

#[derive(Deserialize)]
struct MrEnv {
    client: String,
    server: String,
}

// ---------------------------------------------------------------------------
// Loader key → (loader_name, version)
// ---------------------------------------------------------------------------

const LOADER_KEYS: &[(&str, &str)] = &[
    ("fabric-loader", "fabric"),
    ("quilt-loader", "quilt"),
    ("forge", "forge"),
    ("neoforge", "neoforge"),
    ("neo-forge", "neoforge"),
];

fn extract_loader(deps: &HashMap<String, String>) -> Option<(String, String)> {
    for &(key, name) in LOADER_KEYS {
        if let Some(ver) = deps.get(key) {
            return Some((name.to_string(), ver.clone()));
        }
    }
    None
}

// ---------------------------------------------------------------------------
// Public API
// ---------------------------------------------------------------------------

/// Parse a Modrinth `.mrpack` archive and return a unified [`ModpackManifest`].
///
/// The reader must point to a valid ZIP archive containing `modrinth.index.json`.
pub fn parse_mrpack<R: Read + Seek>(reader: R) -> crate::Result<ModpackManifest> {
    let mut archive = zip::ZipArchive::new(reader)
        .map_err(|e| ContentError::InvalidArchive(format!("failed to open zip: {e}")))?;

    let index_file = archive
        .by_name("modrinth.index.json")
        .map_err(|_| ContentError::InvalidArchive("missing modrinth.index.json".into()))?;

    let index: MrIndex = serde_json::from_reader(index_file)
        .map_err(|e| ContentError::InvalidArchive(format!("invalid modrinth.index.json: {e}")))?;

    let minecraft_version = index
        .dependencies
        .get("minecraft")
        .cloned()
        .ok_or_else(|| {
            ContentError::InvalidArchive("missing minecraft version in dependencies".into())
        })?;

    let (loader, loader_version) = extract_loader(&index.dependencies).unwrap_or_else(|| {
        // Fallback: vanilla pack with no loader
        ("vanilla".to_string(), String::new())
    });

    let files = index
        .files
        .into_iter()
        .map(|f| ModpackFile {
            path: f.path,
            download_urls: f.downloads,
            size: f.file_size,
            hashes: f.hashes,
            required: true,
            env: f.env.map(|e| match (e.client.as_str(), e.server.as_str()) {
                ("required", "unsupported") | ("optional", "unsupported") => {
                    ModpackFileEnv::Client
                }
                ("unsupported", "required") | ("unsupported", "optional") => {
                    ModpackFileEnv::Server
                }
                _ => ModpackFileEnv::Both,
            }),
            project_id: None,
            file_id: None,
        })
        .collect();

    Ok(ModpackManifest {
        name: index.name,
        version: Some(index.version_id),
        author: None,
        minecraft_version,
        loader,
        loader_version,
        files,
        source: ModpackSource::Modrinth,
    })
}
