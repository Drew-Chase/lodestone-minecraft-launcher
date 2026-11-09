//! Which files the server pack takes. The folders the user ticks are walked as usual, except `mods`, which the mod list
//! decides jar by jar. A jar is taken when its choice, or else its detected side, says it runs on the server.

use std::io;
use std::path::{Path, PathBuf};

use sievemc::{ModDetection, Side};

use crate::plan::{Planned, Selection, expand};

/// The folder whose jars the mod list decides. It is never walked: the jars the pack takes are added one by one.
pub const MODS: &str = "mods";

/// The side a user chose for a mod on the Review step. A mod with no choice keeps its detected side.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Choice {
    /// The mod goes into the pack, whatever its detected side.
    Include,
    /// The mod stays out of the pack, whatever its detected side.
    Exclude,
}

/// Whether the pack takes `detection`, given the user's choice for it. With no choice, a client-only mod is left out,
/// and a server-only, both-sides or unsure mod is taken.
pub fn takes(detection: &ModDetection, choice: Option<Choice>) -> bool {
    match choice {
        Some(Choice::Include) => true,
        Some(Choice::Exclude) => false,
        None => !matches!(detection.side, Some(Side::ClientOnly)),
    }
}

/// The key a choice is remembered under: the mod's id when its manifest gives one, else its jar path.
pub fn key(detection: &ModDetection) -> String {
    detection.mod_id.clone().unwrap_or_else(|| detection.path.to_string_lossy().into_owned())
}

/// The files the pack takes from the instance folder at `root`: every file that `folders` takes outside `mods`, plus
/// each jar in `jars` (paths inside `mods`). Sorted by path.
pub fn plan(root: &Path, folders: &Selection, jars: &[PathBuf]) -> io::Result<Vec<Planned>> {
    let mut folders = folders.clone();
    folders.set(MODS, false, |_| Vec::new());
    let mut files = expand(root, &folders)?;
    for jar in jars {
        let Some(name) = jar.file_name() else {
            continue;
        };
        files.push(Planned {
            path: format!("{MODS}/{}", name.to_string_lossy()),
            source: jar.clone(),
            size: std::fs::metadata(jar)?.len(),
        });
    }
    files.sort_by(|a, b| a.path.cmp(&b.path));
    Ok(files)
}

#[cfg(test)]
#[path = "../../tests/server_pack/plan.rs"]
mod tests;
