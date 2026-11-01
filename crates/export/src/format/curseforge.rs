//! The CurseForge pack: a zip with `manifest.json` at the root and an `overrides/` folder. A mod the export
//! knows by its CurseForge project and file is listed by those ids. Everything else is bundled into the
//! overrides, since the manifest has no place for a download.

use std::path::Path;

use serde::Serialize;

use super::{Metadata, write_archive};
use crate::error::Result;
use crate::link::{CurseForgeLink, Entry};
use crate::progress::Control;

/// The manifest's file name.
const MANIFEST: &str = "manifest.json";

/// The manifest: the pack's identity, the Minecraft version and loader, and the mods it lists.
#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
struct Manifest<'a> {
    minecraft: Minecraft<'a>,
    manifest_type: &'static str,
    manifest_version: u32,
    name: &'a str,
    version: &'a str,
    author: &'static str,
    files: Vec<ManifestFile>,
    overrides: &'static str,
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
struct Minecraft<'a> {
    version: &'a str,
    mod_loaders: Vec<ModLoader>,
}

/// A loader, named as CurseForge names it: its key and its version, such as `fabric-0.16.10`.
#[derive(Serialize)]
struct ModLoader {
    id: String,
    primary: bool,
}

/// A mod the pack lists, by its CurseForge ids.
#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
struct ManifestFile {
    #[serde(rename = "projectID")]
    project_id: u64,
    #[serde(rename = "fileID")]
    file_id: u64,
    required: bool,
}

/// Writes the CurseForge pack for `entries` to `target`.
pub fn write(target: &Path, metadata: &Metadata, entries: &[Entry], control: &mut Control<'_>) -> Result<()> {
    let mut files = Vec::new();
    let mut bundled = Vec::new();
    for entry in entries {
        match curseforge_file(entry) {
            Some(file) => files.push(file),
            None => bundled.push(entry),
        }
    }
    let manifest = Manifest {
        minecraft: Minecraft {
            version: metadata.minecraft.as_str(),
            mod_loaders: mod_loaders(metadata),
        },
        manifest_type: "minecraftModpack",
        manifest_version: 1,
        name: metadata.name.as_str(),
        version: metadata.version.as_str(),
        author: "",
        files,
        overrides: "overrides",
    };
    let json = serde_json::to_vec_pretty(&manifest)?;
    write_archive(target, (MANIFEST, &json), &bundled, "overrides/", control)
}

/// Whether the manifest lists `entry`.
pub(crate) fn is_linked(entry: &Entry) -> bool {
    curseforge_file(entry).is_some()
}

/// The manifest entry for `entry`, when its CurseForge project and file are numeric ids.
fn curseforge_file(entry: &Entry) -> Option<ManifestFile> {
    let CurseForgeLink { project, file } = entry.links.curseforge.as_ref()?;
    Some(ManifestFile {
        project_id: project.parse().ok()?,
        file_id: file.parse().ok()?,
        required: true,
    })
}

/// The loader the pack runs on. Vanilla, or a loader with no version, lists none.
fn mod_loaders(metadata: &Metadata) -> Vec<ModLoader> {
    match (metadata.loader.as_str(), metadata.loader_version.as_deref()) {
        ("vanilla", _) | (_, None) => Vec::new(),
        (loader, Some(version)) => vec![ModLoader {
            id: format!("{loader}-{version}"),
            primary: true,
        }],
    }
}

#[cfg(test)]
#[path = "../../tests/format/curseforge.rs"]
mod tests;
