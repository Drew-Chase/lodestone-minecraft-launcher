//! What a paste in the Files tab does, decided from the system's clipboard and from the files the launcher itself
//! remembers copying or cutting.

use std::collections::BTreeSet;
use std::path::{Path, PathBuf};

use crate::app::pages::instance::browser::{Clip, Mode};

/// The entries a paste puts in place, and whether it copies or moves them. `system` is the list of files the system's
/// clipboard holds, when it could be read, and `clip` is what the launcher last copied or cut.
///
/// The system's clipboard decides what is pasted, so files copied in another app paste here. When it holds the same
/// entries the launcher cut, they are moved. When it holds nothing, or could not be read, the launcher's own list is
/// used, so a copy and paste inside the launcher never depends on it. `None` when there is nothing to paste.
pub fn paste_plan(system: Option<Vec<PathBuf>>, clip: Option<&Clip>) -> Option<(Vec<PathBuf>, Mode)> {
    let own_mode = |clip: &Clip| if clip.cut { Mode::Move } else { Mode::Copy };
    match (system.filter(|paths| !paths.is_empty()), clip) {
        (Some(system), Some(clip)) if same_entries(&system, &clip.paths) => Some((clip.paths.clone(), own_mode(clip))),
        (Some(system), _) => Some((system, Mode::Copy)),
        (None, Some(clip)) => Some((clip.paths.clone(), own_mode(clip))),
        (None, None) => None,
    }
}

/// Whether two lists name the same entries, in any order.
fn same_entries(a: &[PathBuf], b: &[PathBuf]) -> bool {
    let (a, b): (BTreeSet<&Path>, BTreeSet<&Path>) = (a.iter().map(PathBuf::as_path).collect(), b.iter().map(PathBuf::as_path).collect());
    a == b
}

#[cfg(test)]
#[path = "../../../tests/app/file_actions/transfer.rs"]
mod tests;
