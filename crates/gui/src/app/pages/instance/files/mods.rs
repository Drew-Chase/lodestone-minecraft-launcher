//! The mod jars in `mods/`, each read for the metadata it declares.

use std::collections::HashSet;
use std::path::{Path, PathBuf};
use std::sync::Arc;
use std::time::UNIX_EPOCH;

use iced::widget::image;
use lodestone_loaders::{Fabric, Loader, ModDependency};

use super::mod_index::{self, JarKey, Read};

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
    /// The id the mod declares. The dependencies of other mods name it.
    pub id: Option<String>,
    /// The dependencies the mod declares, each with the id it names and how the mod needs it.
    pub dependencies: Vec<ModDependency>,
    /// The icon the mod declares in its jar, when it declares one that can be read.
    pub icon: Option<image::Handle>,
}

/// A jar found in `mods/`, before its metadata is read.
struct Jar {
    path: PathBuf,
    file_name: String,
    /// The file name without `.jar` or `.jar.disabled`.
    stem: String,
    enabled: bool,
}

/// Reads every mod jar in `dir`, sorted by name. A jar whose metadata cannot be read is still listed, under its file name.
///
/// Only the jars the index does not know yet are opened, several at once (see [`mod_index`]), so listing a folder again,
/// or listing it after a mod is switched on or off, reads nothing but the folder itself.
pub fn scan(dir: &Path) -> Vec<ModFile> {
    let jars: Vec<(Jar, JarKey)> = jars(dir)
        .into_iter()
        .filter_map(|jar| {
            let key = key_of(&jar)?;
            Some((jar, key))
        })
        .collect();
    let mut known = mod_index::entries(dir);
    let missing: Vec<(&Path, &JarKey)> = jars
        .iter()
        .filter(|(_, key)| !known.contains_key(key))
        .map(|(jar, key)| (jar.path.as_path(), key))
        .collect();
    let mut added = Vec::new();
    for (key, read) in read_jars(&missing) {
        added.push((key.clone(), read.icon_png));
        known.insert(key, Arc::new(read.known));
    }
    // Entries of jars that are gone leave the index.
    let held: HashSet<&JarKey> = jars.iter().map(|(_, key)| key).collect();
    let removed: Vec<JarKey> = known.keys().filter(|key| !held.contains(key)).cloned().collect();
    for key in &removed {
        known.remove(key);
    }

    let mut mods: Vec<ModFile> = jars
        .iter()
        .map(|(jar, key)| {
            let entry = known.get(key);
            ModFile {
                name: entry.and_then(|entry| entry.name.clone()).unwrap_or_else(|| jar.stem.clone()),
                author: entry.and_then(|entry| entry.author.clone()),
                id: entry.and_then(|entry| entry.id.clone()),
                version: entry.and_then(|entry| entry.version.clone()),
                dependencies: entry.map(|entry| entry.dependencies.clone()).unwrap_or_default(),
                icon: entry.and_then(|entry| entry.icon.clone()),
                file_name: jar.file_name.clone(),
                enabled: jar.enabled,
            }
        })
        .collect();
    mod_index::store(dir, known, added, removed);
    mods.sort_by_key(|entry| entry.name.to_lowercase());
    mods
}

/// How a jar is known to the index, from its file's size and modification time. `None` when the file cannot be read.
fn key_of(jar: &Jar) -> Option<JarKey> {
    let metadata = std::fs::metadata(&jar.path).ok()?;
    let modified = metadata
        .modified()
        .ok()
        .and_then(|time| time.duration_since(UNIX_EPOCH).ok())
        .map_or(0, |since| u64::try_from(since.as_nanos()).unwrap_or(u64::MAX));
    Some(JarKey {
        stem: jar.stem.clone(),
        size: metadata.len(),
        modified,
    })
}

/// Reads the jars across the machine's cores. Each thread takes an equal share; a few jars take one thread.
fn read_jars(jars: &[(&Path, &JarKey)]) -> Vec<(JarKey, Read)> {
    const PER_THREAD: usize = 4;
    const MOST_THREADS: usize = 16;
    if jars.is_empty() {
        return Vec::new();
    }
    // Vanilla declares no mod metadata, so the Fabric kind reads the jars of every loader: the detected format wins,
    // whichever kind asked.
    let loader = Loader::<Fabric>::new();
    let threads = std::thread::available_parallelism()
        .map_or(1, std::num::NonZero::get)
        .min(MOST_THREADS)
        .min(jars.len().div_ceil(PER_THREAD));
    let read_share = |share: &[(&Path, &JarKey)]| -> Vec<(JarKey, Read)> {
        share.iter().map(|(path, key)| ((*key).clone(), mod_index::read(&loader, path))).collect()
    };
    if threads <= 1 {
        return read_share(jars);
    }
    let chunk = jars.len().div_ceil(threads);
    std::thread::scope(|scope| {
        let workers: Vec<_> = jars.chunks(chunk).map(|share| scope.spawn(move || read_share(share))).collect();
        workers.into_iter().filter_map(|worker| worker.join().ok()).flatten().collect()
    })
}

/// How many mod jars `dir` holds, enabled or not.
pub fn count(dir: &Path) -> usize {
    jars(dir).len()
}

/// The file name a mod jar takes when its switch is set to `enabled`. A jar switched off gains `.disabled`, and one
/// switched on loses it. `None` when the name is not a mod jar, or already has the state asked for.
pub fn switched_name(file_name: &str, enabled: bool) -> Option<String> {
    if enabled {
        let stem = strip_suffix_ignore_case(file_name, ".disabled")?;
        strip_suffix_ignore_case(stem, ".jar")?;
        Some(stem.to_string())
    } else {
        strip_suffix_ignore_case(file_name, ".jar")?;
        Some(format!("{file_name}.disabled"))
    }
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
