//! Modpack archive parsing and extraction.
//!
//! Supports:
//! - **Modrinth `.mrpack`** — ZIP with `modrinth.index.json`
//! - **CurseForge** — ZIP with `manifest.json`

mod curseforge_pack;
mod manifest;
mod mrpack;
mod overrides;

pub use curseforge_pack::parse_curseforge_pack;
pub use manifest::{ModpackFile, ModpackFileEnv, ModpackManifest, ModpackSource};
pub use mrpack::parse_mrpack;
pub use overrides::extract_overrides;

use std::io::{Read, Seek};

use crate::error::ContentError;

/// Auto-detect the modpack format and parse the archive.
///
/// Tries Modrinth first (`modrinth.index.json`), then CurseForge (`manifest.json`).
/// Returns [`ContentError::InvalidArchive`] if neither is found.
pub fn parse_modpack<R: Read + Seek>(reader: R) -> crate::Result<ModpackManifest> {
    let mut archive = zip::ZipArchive::new(reader)
        .map_err(|e| ContentError::InvalidArchive(format!("failed to open zip: {e}")))?;

    let is_mrpack = archive.by_name("modrinth.index.json").is_ok();
    let is_cf = !is_mrpack && archive.by_name("manifest.json").is_ok();

    // We need to pass the underlying reader to the format-specific parsers,
    // so consume the archive and recover the reader.
    let reader = archive.into_inner();

    if is_mrpack {
        parse_mrpack(reader)
    } else if is_cf {
        parse_curseforge_pack(reader)
    } else {
        Err(ContentError::InvalidArchive(
            "archive contains neither modrinth.index.json nor manifest.json".into(),
        ))
    }
}
