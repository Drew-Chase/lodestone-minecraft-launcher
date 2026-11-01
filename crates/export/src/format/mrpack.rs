//! The Modrinth pack: a zip with `modrinth.index.json` at the root, and an `overrides/` folder holding every
//! file the index does not place. A file goes in the index when Modrinth has it and the export knows its
//! download and both digests.

use std::collections::BTreeMap;
use std::path::Path;

use serde::Serialize;

use super::{Metadata, write_archive};
use crate::error::Result;
use crate::link::Entry;
use crate::progress::Control;

/// The index's file name.
const INDEX: &str = "modrinth.index.json";

/// The index: the pack's identity, the files it places, and the versions it needs.
#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
struct Index<'a> {
    format_version: u32,
    game: &'static str,
    version_id: &'a str,
    name: &'a str,
    #[serde(skip_serializing_if = "Option::is_none")]
    summary: Option<&'a str>,
    files: Vec<IndexFile<'a>>,
    dependencies: BTreeMap<&'static str, &'a str>,
    /// Where the overrides folder is. The launcher reads this key to know the folder holds files to lay over the instance.
    overrides: &'static str,
}

/// One file the index places: where it goes, its hashes, where to get it, and which sides run it.
#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
struct IndexFile<'a> {
    path: &'a str,
    hashes: Hashes<'a>,
    env: Env,
    downloads: Vec<&'a str>,
    file_size: u64,
}

#[derive(Serialize)]
struct Hashes<'a> {
    sha1: &'a str,
    sha512: &'a str,
}

#[derive(Serialize)]
struct Env {
    client: &'static str,
    server: &'static str,
}

/// Writes the Modrinth pack for `entries` to `target`.
pub fn write(target: &Path, metadata: &Metadata, entries: &[Entry], control: &mut Control<'_>) -> Result<()> {
    let mut files = Vec::new();
    let mut bundled = Vec::new();
    for entry in entries {
        match index_file(entry) {
            Some(file) => files.push(file),
            None => bundled.push(entry),
        }
    }
    let index = Index {
        format_version: 1,
        game: "minecraft",
        version_id: metadata.version.as_str(),
        name: metadata.name.as_str(),
        summary: (!metadata.description.is_empty()).then_some(metadata.description.as_str()),
        files,
        dependencies: dependencies(metadata),
        overrides: "overrides",
    };
    let json = serde_json::to_vec_pretty(&index)?;
    write_archive(target, (INDEX, &json), &bundled, "overrides/", control)
}

/// Whether the index places `entry`.
pub(crate) fn is_linked(entry: &Entry) -> bool {
    index_file(entry).is_some()
}

/// The index entry for `entry`, when Modrinth has the file and the export knows its digests.
fn index_file(entry: &Entry) -> Option<IndexFile<'_>> {
    let digests = entry.digests.as_ref()?;
    let modrinth = entry.links.modrinth.as_ref().filter(|link| !link.download_url.is_empty())?;
    Some(IndexFile {
        path: entry.path.as_str(),
        hashes: Hashes {
            sha1: digests.sha1.as_str(),
            sha512: digests.sha512.as_str(),
        },
        env: Env {
            client: "required",
            server: "required",
        },
        downloads: vec![modrinth.download_url.as_str()],
        file_size: entry.size,
    })
}

/// The versions the pack needs: the Minecraft version, and the loader's version under its Modrinth key.
fn dependencies(metadata: &Metadata) -> BTreeMap<&'static str, &str> {
    let mut dependencies = BTreeMap::new();
    dependencies.insert("minecraft", metadata.minecraft.as_str());
    if let (Some(key), Some(version)) = (loader_key(&metadata.loader), metadata.loader_version.as_deref()) {
        dependencies.insert(key, version);
    }
    dependencies
}

/// The key Modrinth names a loader's version by. Vanilla has none.
fn loader_key(loader: &str) -> Option<&'static str> {
    match loader {
        "fabric" => Some("fabric-loader"),
        "quilt" => Some("quilt-loader"),
        "forge" => Some("forge"),
        "neoforge" => Some("neoforge"),
        _ => None,
    }
}

#[cfg(test)]
#[path = "../../tests/format/mrpack.rs"]
mod tests;
