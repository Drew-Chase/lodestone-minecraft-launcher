//! What a modpack's release leaves in an instance, kept for later: the files it shipped, with their hashes, and the pristine
//! copies of its text files that a three-way update merges against. The copies sit outside the instance folder, so an export
//! and the instance's file watcher never see them.

use std::collections::BTreeMap;
use std::path::Path;

use tracing::warn;

use super::modpack::Payload;
use crate::app::mod_sources::sha1_of_file;
use crate::app::shared_library::PackFile;

/// The largest text file whose pristine copy is kept. A larger file that conflicts is merged as a whole, not line by line.
pub(crate) const COPY_LIMIT: u64 = 256 * 1024;

/// How many leading bytes are read to tell a text file from a binary one. A NUL byte among them makes the file binary.
const SNIFF: usize = 8 * 1024;

/// A folder the user keeps for their own content. A release ships files into it, but an update only adds the ones that are
/// missing and never replaces one that is there.
pub(crate) fn user_folder(path: &str) -> bool {
    const FOLDERS: [&str; 5] = ["saves/", "screenshots/", "logs/", "resourcepacks/", "shaderpacks/"];
    FOLDERS.iter().any(|folder| path.starts_with(folder))
}

/// A file the user edits in the game, or the launcher keeps for the instance. No update replaces one that is there.
pub(crate) fn user_file(path: &str) -> bool {
    const FILES: [&str; 4] = ["options.txt", "optionsof.txt", "optionsshaders.txt", "servers.dat"];
    path.starts_with(".lodestone/") || FILES.contains(&path)
}

/// Whether a pristine copy is kept for `path`: a text file of the release's own, outside the mods, the user's folders and the
/// user's files.
fn keeps_copy(path: &str) -> bool {
    !path.starts_with("mods/") && !user_folder(path) && !user_file(path)
}

/// The files an install leaves in the instance: each file its release shipped, with its SHA-1. The hash the release gives is
/// used when it gives one, and the hash read from disk otherwise. `shipped` lists the files the archives wrote, and the
/// payload's own files are added to them. The hashing runs on the blocking pool.
pub(crate) async fn record(game_dir: &Path, instance: &str, payload: Option<&Payload>, shipped: Vec<String>) -> Result<Vec<PackFile>, String> {
    let given: Vec<(String, Option<String>)> = payload
        .map(|payload| payload.files.iter().map(|file| (file.path.clone(), file.sha1.clone())).collect())
        .unwrap_or_default();
    let game_dir = game_dir.to_path_buf();
    let instance = instance.to_string();
    tokio::task::spawn_blocking(move || {
        let mut hashes: BTreeMap<String, Option<String>> = BTreeMap::new();
        for (path, sha1) in given {
            hashes.insert(path, sha1.map(|sha1| sha1.to_ascii_lowercase()));
        }
        for path in shipped {
            hashes.entry(path).or_insert(None);
        }
        hashes
            .into_iter()
            .map(|(path, sha1)| {
                let sha1 = sha1.or_else(|| sha1_of_file(&game_dir.join(&path)).ok());
                PackFile {
                    instance: instance.clone(),
                    path,
                    sha1,
                }
            })
            .collect::<Vec<PackFile>>()
    })
    .await
    .map_err(|error| error.to_string())
}

/// Keeps a pristine copy, under `base`, of each text file in `files` that is small enough and outside the mods and the user's
/// own files. Each copy is read from the instance folder as the release left it. A file that cannot be read is skipped.
pub(crate) async fn write_base(base: &Path, game_dir: &Path, files: &[PackFile]) -> Result<(), String> {
    let wanted: Vec<String> = files.iter().filter(|file| keeps_copy(&file.path)).map(|file| file.path.clone()).collect();
    let (base, game_dir) = (base.to_path_buf(), game_dir.to_path_buf());
    tokio::task::spawn_blocking(move || -> Result<(), String> {
        for path in wanted {
            let source = game_dir.join(&path);
            let Ok(meta) = std::fs::metadata(&source) else {
                continue;
            };
            if !meta.is_file() || meta.len() > COPY_LIMIT {
                continue;
            }
            let bytes = match std::fs::read(&source) {
                Ok(bytes) => bytes,
                Err(error) => {
                    warn!("could not read {} for its pristine copy: {error}", source.display());
                    continue;
                }
            };
            if !is_text(&bytes) {
                continue;
            }
            let target = base.join(&path);
            if let Some(parent) = target.parent() {
                std::fs::create_dir_all(parent).map_err(|error| error.to_string())?;
            }
            lodestone_fs::write_atomic(&target, &bytes).map_err(|error| error.to_string())?;
        }
        Ok(())
    })
    .await
    .map_err(|error| error.to_string())?
}

/// Whether `bytes` read as text: no NUL byte among the first few kilobytes.
pub(crate) fn is_text(bytes: &[u8]) -> bool {
    !bytes[..bytes.len().min(SNIFF)].contains(&0)
}

#[cfg(test)]
#[path = "../../../tests/app/install/shipped.rs"]
mod tests;
