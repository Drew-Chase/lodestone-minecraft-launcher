//! Which files an export takes from the instance folder. The user turns paths off, and the [`Selection`]
//! keeps that set; [`expand`] walks the folder and lists what is left. Paths are relative to the instance
//! folder and use `/` on every platform.

use std::collections::BTreeSet;
use std::io;
use std::path::{Path, PathBuf};

/// The top-level entries a new export leaves out: logs, crash reports, worlds and screenshots.
pub const DEFAULT_EXCLUDED: [&str; 4] = ["logs", "crash-reports", "saves", "screenshots"];

/// Whether `name` is a marker an install leaves in an instance folder while it runs. Those are never exported.
pub fn is_leftover(name: &str) -> bool {
    matches!(name, ".installing" | ".modpack")
}

/// Whether a row is checked, unchecked, or checked with something under it turned off.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CheckState {
    Checked,
    Partial,
    Unchecked,
}

/// The paths the user turned off. A path is taken unless it, or an ancestor of it, is in the set.
#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct Selection {
    excluded: BTreeSet<String>,
}

impl Selection {
    /// The selection a new export starts with: everything except [`DEFAULT_EXCLUDED`].
    pub fn new_export() -> Self {
        Self::from_excluded(DEFAULT_EXCLUDED.iter().map(|name| name.to_string()))
    }

    /// A selection with the given paths turned off.
    pub fn from_excluded(excluded: impl IntoIterator<Item = String>) -> Self {
        Self {
            excluded: excluded.into_iter().collect(),
        }
    }

    /// Whether `path` is taken.
    pub fn is_included(&self, path: &str) -> bool {
        !self.excluded.iter().any(|excluded| is_within(path, excluded))
    }

    /// The check state of the row at `path`.
    pub fn state(&self, path: &str) -> CheckState {
        if !self.is_included(path) {
            CheckState::Unchecked
        } else if self.excluded.iter().any(|excluded| is_within(excluded, path)) {
            CheckState::Partial
        } else {
            CheckState::Checked
        }
    }

    /// Turns `path` on or off. Turning a folder on turns on everything under it. Turning a path on under an
    /// excluded folder turns that folder on too, and turns off its other children. `children` lists the
    /// immediate children of a folder, so those can be turned off; a folder it does not know lists none.
    pub fn set(&mut self, path: &str, included: bool, children: impl Fn(&str) -> Vec<String>) {
        self.excluded.retain(|excluded| !is_within(excluded, path));
        if !included {
            if self.is_included(path) {
                self.excluded.insert(path.to_string());
            }
            return;
        }
        while let Some(blocker) = self.blocker(path) {
            self.excluded.remove(&blocker);
            let next = child_towards(&blocker, path);
            for child in children(&blocker) {
                if child != next {
                    self.excluded.insert(child);
                }
            }
        }
    }

    /// The excluded path nearest the top of the tree that `path` sits under.
    fn blocker(&self, path: &str) -> Option<String> {
        self.excluded
            .iter()
            .filter(|excluded| is_within(path, excluded))
            .min_by_key(|excluded| excluded.len())
            .cloned()
    }
}

/// Whether `path` is `ancestor` or sits below it.
fn is_within(path: &str, ancestor: &str) -> bool {
    path == ancestor || path.strip_prefix(ancestor).is_some_and(|rest| rest.starts_with('/'))
}

/// The child of `ancestor` on the way to `path`. `path` must sit below `ancestor`.
fn child_towards(ancestor: &str, path: &str) -> String {
    let rest = &path[ancestor.len() + 1..];
    let first = rest.split('/').next().unwrap_or(rest);
    format!("{ancestor}/{first}")
}

/// A file the export takes, with the place it is read from.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Planned {
    /// The path inside the instance, with `/` separators.
    pub path: String,
    /// Where the bytes are on disk.
    pub source: PathBuf,
    /// The file's size in bytes.
    pub size: u64,
}

/// Lists the files `selection` takes from the instance folder at `root`, sorted by path. Symbolic links are
/// skipped, and so are the install's leftovers and every folder the selection turns off.
pub fn expand(root: &Path, selection: &Selection) -> io::Result<Vec<Planned>> {
    let mut planned = Vec::new();
    walk(root, "", selection, &mut planned)?;
    planned.sort_by(|a, b| a.path.cmp(&b.path));
    Ok(planned)
}

fn walk(folder: &Path, prefix: &str, selection: &Selection, planned: &mut Vec<Planned>) -> io::Result<()> {
    for entry in std::fs::read_dir(folder)? {
        let entry = entry?;
        let name = entry.file_name().to_string_lossy().into_owned();
        if is_leftover(&name) {
            continue;
        }
        let path = if prefix.is_empty() { name } else { format!("{prefix}/{name}") };
        if !selection.is_included(&path) {
            continue;
        }
        let file_type = entry.file_type()?;
        if file_type.is_symlink() {
            continue;
        }
        let source = entry.path();
        if file_type.is_dir() {
            walk(&source, &path, selection, planned)?;
        } else if file_type.is_file() {
            let size = entry.metadata()?.len();
            planned.push(Planned { path, source, size });
        }
    }
    Ok(())
}

#[cfg(test)]
#[path = "../tests/plan.rs"]
mod tests;
