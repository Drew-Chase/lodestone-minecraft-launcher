//! The mod jars in `mods/`, each read for the metadata it declares.

use std::path::{Path, PathBuf};

use lodestone_loaders::{Fabric, Loader};

/// One mod jar.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ModFile {
    /// The file name on disk, `.disabled` included.
    pub file_name: String,
    /// `*.jar` is active; `*.jar.disabled` is not.
    pub enabled: bool,
    /// The name the mod declares, or the file name without its extension.
    pub name: String,
    /// The first author the mod declares.
    pub author: Option<String>,
    /// The version the mod declares.
    pub version: Option<String>,
}

/// A jar found in `mods/`, before its metadata is read.
struct Jar {
    path: PathBuf,
    file_name: String,
    /// The file name without `.jar` or `.jar.disabled`.
    stem: String,
    enabled: bool,
}

/// Reads every mod jar in `dir`, sorted by name. A jar whose metadata cannot be
/// read is still listed, under its file name.
pub fn scan(dir: &Path) -> Vec<ModFile> {
    // Vanilla declares no mod metadata, so the Fabric kind reads the jars of
    // every loader: the detected format wins, whichever kind asked.
    let loader = Loader::<Fabric>::new();
    let mut mods: Vec<ModFile> = jars(dir)
        .into_iter()
        .map(|jar| {
            let info = loader.extract_mod_info(&jar.path).ok();
            let name = info
                .as_ref()
                .and_then(|info| info.name.clone())
                .filter(|name| !name.trim().is_empty())
                .unwrap_or_else(|| jar.stem.clone());
            ModFile {
                author: info.as_ref().and_then(|info| info.authors.first().cloned()),
                version: info.and_then(|info| info.version),
                file_name: jar.file_name,
                enabled: jar.enabled,
                name,
            }
        })
        .collect();
    mods.sort_by_key(|entry| entry.name.to_lowercase());
    mods
}

/// How many mod jars `dir` holds, enabled or not.
pub fn count(dir: &Path) -> usize {
    jars(dir).len()
}

/// The jars directly inside `dir`. Folders and other files are skipped.
fn jars(dir: &Path) -> Vec<Jar> {
    let Ok(entries) = std::fs::read_dir(dir) else {
        return Vec::new();
    };
    entries
        .filter_map(Result::ok)
        .filter(|entry| entry.file_type().is_ok_and(|kind| kind.is_file()))
        .filter_map(|entry| {
            let file_name = entry.file_name().to_string_lossy().into_owned();
            let (stem, enabled) = match strip_suffix_ignore_case(&file_name, ".jar.disabled") {
                Some(stem) => (stem.to_string(), false),
                None => (strip_suffix_ignore_case(&file_name, ".jar")?.to_string(), true),
            };
            Some(Jar {
                path: entry.path(),
                file_name,
                stem,
                enabled,
            })
        })
        .collect()
}

/// `name` without `suffix` when it ends with it, ignoring ASCII case. The
/// comparison runs on bytes, so a multi-byte name can never split a character.
fn strip_suffix_ignore_case<'a>(name: &'a str, suffix: &str) -> Option<&'a str> {
    let split = name.len().checked_sub(suffix.len())?;
    let tail = name.as_bytes().get(split..)?;
    tail.eq_ignore_ascii_case(suffix.as_bytes()).then(|| &name[..split])
}

#[cfg(test)]
#[path = "../../../../../tests/app/pages/instance/files/mods.rs"]
mod tests;
