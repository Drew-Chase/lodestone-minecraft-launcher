//! The icon a mod jar declares. Each loader names its icon in its own metadata, so the icon is found from
//! those files directly. That also works for a jar whose metadata does not parse: a Fabric `icon` written as
//! a map of sizes fails modcrawl's parse, and the jar's picture is still wanted.

use std::io::Read;
use std::path::Path;

use serde_json::Value;
use zipcrawl::ZipManager;

/// The largest icon read, in bytes. A mod's icon is a small square picture.
const MAX_BYTES: u64 = 2 * 1024 * 1024;

/// The bytes of the picture a jar declares as its icon, or `None` when the jar declares none, the file it
/// names is missing or too large, or the jar cannot be read.
pub(crate) fn read(jar: &Path) -> Option<Vec<u8>> {
    let mut archive = ZipManager::new(jar).ok()?;
    let path = declared_path(&mut archive)?;
    let mut bytes = Vec::new();
    archive.open_file(&path).ok()?.take(MAX_BYTES + 1).read_to_end(&mut bytes).ok()?;
    (bytes.len() as u64 <= MAX_BYTES).then_some(bytes)
}

/// The path the jar's metadata names for its icon. Fabric names it in `fabric.mod.json`, Quilt in
/// `quilt.mod.json`, and Forge and NeoForge in their `mods.toml`. The first file that names one wins.
fn declared_path(archive: &mut ZipManager) -> Option<String> {
    let fabric = json(archive, "fabric.mod.json").and_then(|document| largest(document.get("icon")?));
    let path = fabric
        .or_else(|| json(archive, "quilt.mod.json").and_then(|document| largest(document.get("quilt_loader")?.get("metadata")?.get("icon")?)))
        .or_else(|| forge(archive))?;
    Some(path.trim_start_matches('/').to_string())
}

/// A JSON metadata file, or `None` when it is missing or does not parse.
fn json(archive: &mut ZipManager, name: &str) -> Option<Value> {
    let text = archive.read_to_string(name).ok()?;
    serde_json::from_str(&text).ok()
}

/// The `logoFile` a `mods.toml` names at its top level. NeoForge's file is read first.
fn forge(archive: &mut ZipManager) -> Option<String> {
    ["META-INF/neoforge.mods.toml", "META-INF/mods.toml"].into_iter().find_map(|name| {
        let text = archive.read_to_string(name).ok()?;
        let table: toml::Table = text.parse().ok()?;
        table.get("logoFile")?.as_str().map(str::to_string)
    })
}

/// A declared icon: a path, or a map from a size to a path. The largest size wins, since the icon is drawn
/// small and the larger picture scales down cleanly.
fn largest(icon: &Value) -> Option<String> {
    match icon {
        Value::String(path) => Some(path.clone()),
        Value::Object(sizes) => sizes
            .iter()
            .filter_map(|(size, path)| Some((size.parse::<u32>().ok()?, path.as_str()?)))
            .max_by_key(|(size, _)| *size)
            .map(|(_, path)| path.to_string()),
        _ => None,
    }
}

#[cfg(test)]
#[path = "../../tests/mod_info/icon.rs"]
mod tests;
