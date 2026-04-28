use std::collections::HashMap;
use std::io::{Read, Seek};

use serde::Deserialize;

use super::manifest::{ModpackFile, ModpackManifest, ModpackSource};
use crate::error::ContentError;

// ---------------------------------------------------------------------------
// CurseForge manifest DTOs
// ---------------------------------------------------------------------------

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
struct CfManifest {
    minecraft: CfMinecraft,
    #[allow(dead_code)]
    manifest_type: Option<String>,
    #[allow(dead_code)]
    manifest_version: Option<u32>,
    name: String,
    version: Option<String>,
    author: Option<String>,
    files: Vec<CfPackFile>,
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
struct CfMinecraft {
    version: String,
    mod_loaders: Vec<CfModLoader>,
}

#[derive(Deserialize)]
struct CfModLoader {
    id: String,
    #[allow(dead_code)]
    primary: Option<bool>,
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
struct CfPackFile {
    #[serde(alias = "projectID")]
    project_id: u64,
    #[serde(alias = "fileID")]
    file_id: u64,
    required: bool,
}

// ---------------------------------------------------------------------------
// Loader parsing
// ---------------------------------------------------------------------------

/// Parse a CurseForge modLoader id like `"forge-47.3.0"` into `("forge", "47.3.0")`.
fn parse_loader_id(id: &str) -> (String, String) {
    // Known prefixes to try, longest first to avoid false matches.
    let prefixes = ["neoforge-", "forge-", "fabric-", "quilt-"];
    for prefix in prefixes {
        if let Some(version) = id.strip_prefix(prefix) {
            let name = prefix.trim_end_matches('-');
            return (name.to_string(), version.to_string());
        }
    }
    // Fallback: split on first '-'
    if let Some(pos) = id.find('-') {
        (id[..pos].to_string(), id[pos + 1..].to_string())
    } else {
        (id.to_string(), String::new())
    }
}

// ---------------------------------------------------------------------------
// Public API
// ---------------------------------------------------------------------------

/// Parse a CurseForge modpack ZIP archive and return a unified [`ModpackManifest`].
///
/// Download URLs are **not** resolved here — `ModpackFile::download_urls` will be
/// empty. Use [`CurseForgeProvider::resolve_pack_files`] to populate them.
pub fn parse_curseforge_pack<R: Read + Seek>(reader: R) -> crate::Result<ModpackManifest> {
    let mut archive = zip::ZipArchive::new(reader)
        .map_err(|e| ContentError::InvalidArchive(format!("failed to open zip: {e}")))?;

    let manifest_file = archive
        .by_name("manifest.json")
        .map_err(|_| ContentError::InvalidArchive("missing manifest.json".into()))?;

    let manifest: CfManifest = serde_json::from_reader(manifest_file)
        .map_err(|e| ContentError::InvalidArchive(format!("invalid manifest.json: {e}")))?;

    let (loader, loader_version) = manifest
        .minecraft
        .mod_loaders
        .first()
        .map(|ml| parse_loader_id(&ml.id))
        .unwrap_or_else(|| ("vanilla".to_string(), String::new()));

    let files = manifest
        .files
        .into_iter()
        .map(|f| ModpackFile {
            // CurseForge packs don't specify the file path; it'll be determined
            // after resolution from the API response filename.
            path: String::new(),
            download_urls: Vec::new(),
            size: None,
            hashes: HashMap::new(),
            required: f.required,
            env: None,
            project_id: Some(f.project_id.to_string()),
            file_id: Some(f.file_id.to_string()),
        })
        .collect();

    Ok(ModpackManifest {
        name: manifest.name,
        version: manifest.version,
        author: manifest.author,
        minecraft_version: manifest.minecraft.version,
        loader,
        loader_version,
        files,
        source: ModpackSource::CurseForge,
    })
}
