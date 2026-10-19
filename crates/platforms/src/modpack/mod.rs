//! Modpack archive parsing: `.mrpack` (Modrinth) and CurseForge exports.
//!
//! Both are zips carrying a JSON manifest plus an overrides directory. The
//! archive is sniffed for the manifest entry (via zipcrawl), parsed, and
//! mapped onto [`crate::meta::Modpack`].

use std::path::Path;

use zipcrawl::ZipManager;

use crate::error::PlatformError;
use crate::meta::Modpack;

mod curseforge;
mod mrpack;

/// The manifest entries this crate recognizes, in sniffing order.
const MRPACK_MANIFEST: &str = "modrinth.index.json";
const CURSEFORGE_MANIFEST: &str = "manifest.json";

/// Sniffs the archive for a known manifest and parses it.
pub(crate) fn parse(archive: &Path) -> Result<Modpack, PlatformError> {
    let mut manager = ZipManager::new(archive).map_err(|source| PlatformError::ModpackParse {
        path: archive.to_path_buf(),
        source: Box::new(source),
    })?;

    let entries = manager.entries().map_err(|source| PlatformError::ModpackParse {
        path: archive.to_path_buf(),
        source: Box::new(source),
    })?;
    let names: Vec<&str> = entries.iter().map(|entry| entry.name.as_str()).collect();

    if names.contains(&MRPACK_MANIFEST) {
        let raw = manager.read_to_string(MRPACK_MANIFEST).map_err(|source| PlatformError::ModpackParse {
            path: archive.to_path_buf(),
            source: Box::new(source),
        })?;
        mrpack::parse(&raw, archive)
    } else if names.contains(&CURSEFORGE_MANIFEST) {
        let raw = manager
            .read_to_string(CURSEFORGE_MANIFEST)
            .map_err(|source| PlatformError::ModpackParse {
                path: archive.to_path_buf(),
                source: Box::new(source),
            })?;
        curseforge::parse(&raw, archive)
    } else {
        Err(PlatformError::ModpackFormatUnknown { path: archive.to_path_buf() })
    }
}
