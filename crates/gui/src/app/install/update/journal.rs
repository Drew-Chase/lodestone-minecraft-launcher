//! The journal of an in-place update: the files the update moved aside and the files it created, written before the instance
//! changes, so an update that stops part way can be undone. The workspace holds the journal, the files the update staged, and
//! the originals it moved aside. It sits under the data root, beside the instance folders, so the instance's own folder and
//! its exports never see it.

use std::collections::BTreeMap;
use std::io::ErrorKind;
use std::path::{Path, PathBuf};

use serde::{Deserialize, Serialize};
use tracing::warn;

use crate::app::paths::LauncherPaths;

/// The journal's file name in a workspace.
const JOURNAL: &str = "journal.json";

/// What an update changed in the instance folder, as far as undoing it needs.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub(crate) struct Journal {
    /// Names the update. The database records the token of the update it committed, so a launch that finds the journal can tell
    /// whether the commit happened.
    pub token: String,
    /// The paths, relative to the instance folder, the update moved aside. Each original waits in the workspace's `moved` folder.
    pub moved: Vec<String>,
    /// The paths the update created, which did not exist before it.
    pub created: Vec<String>,
}

/// The folders of one update, under `pack-update/<instance id>/`.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct Workspace {
    root: PathBuf,
    instance: PathBuf,
    base: PathBuf,
}

impl Workspace {
    /// The workspace at `root` for the update of the instance folder `instance`, whose pristine copies live in `base`.
    pub(crate) fn new(root: PathBuf, instance: PathBuf, base: PathBuf) -> Self {
        Self { root, instance, base }
    }

    /// The folder the release's files are staged in, which mirrors the instance folder.
    pub(crate) fn staging(&self) -> PathBuf {
        self.root.join("staging")
    }

    /// The folder the originals the update moved aside wait in, which mirrors the instance folder.
    pub(crate) fn moved(&self) -> PathBuf {
        self.root.join("moved")
    }

    /// The folder the new pristine copies are written to. They replace the instance's copies only once the update commits.
    pub(crate) fn base_new(&self) -> PathBuf {
        self.root.join("base-new")
    }

    /// The instance folder the update changes.
    pub(crate) fn instance(&self) -> &Path {
        &self.instance
    }

    /// The folder of the instance's pristine copies.
    pub(crate) fn base(&self) -> &Path {
        &self.base
    }

    /// Makes the workspace's folders.
    pub(crate) fn create(&self) -> Result<(), String> {
        std::fs::create_dir_all(self.staging()).map_err(|error| error.to_string())
    }

    /// Writes the journal. It is written before the instance changes, so every change it names can be undone.
    pub(crate) fn write_journal(&self, journal: &Journal) -> Result<(), String> {
        let bytes = serde_json::to_vec_pretty(journal).map_err(|error| error.to_string())?;
        lodestone_fs::write_atomic(&self.root.join(JOURNAL), &bytes).map_err(|error| error.to_string())
    }

    /// The journal, when the update wrote one. An update that stopped before it changed anything has none.
    pub(crate) fn read_journal(&self) -> Result<Option<Journal>, String> {
        match std::fs::read(self.root.join(JOURNAL)) {
            Ok(bytes) => serde_json::from_slice(&bytes).map(Some).map_err(|error| error.to_string()),
            Err(error) if error.kind() == ErrorKind::NotFound => Ok(None),
            Err(error) => Err(error.to_string()),
        }
    }

    /// Undoes the changes the journal names, then removes the workspace. A path the journal names that is not there is skipped,
    /// since the update may have stopped before reaching it.
    pub(crate) fn roll_back(&self) -> Result<(), String> {
        if let Some(journal) = self.read_journal()? {
            for path in &journal.created {
                remove_file(&self.instance.join(path))?;
            }
            for path in &journal.moved {
                let kept = self.moved().join(path);
                if !kept.is_file() {
                    continue;
                }
                remove_file(&self.instance.join(path))?;
                move_file(&kept, &self.instance.join(path))?;
            }
        }
        remove_dir(&self.root)
    }

    /// Finishes a committed update: the originals it replaced are discarded, and the new pristine copies take the place of the
    /// old ones. A workspace with no new copies only loses its originals.
    pub(crate) fn finish(&self) -> Result<(), String> {
        if self.base_new().is_dir() {
            remove_dir(&self.base)?;
            if let Some(parent) = self.base.parent() {
                std::fs::create_dir_all(parent).map_err(|error| error.to_string())?;
            }
            std::fs::rename(self.base_new(), &self.base).map_err(|error| error.to_string())?;
        }
        remove_dir(&self.root)
    }
}

impl Workspace {
    /// Removes the workspace of an update that stopped before it wrote its journal, once a staged download still stopping has had
    /// a moment to finish. A workspace with a journal is left for the rollback or the next launch, since the instance may have changed.
    pub(crate) fn discard_unwritten(&self) {
        let root = self.root.clone();
        std::thread::spawn(move || {
            std::thread::sleep(std::time::Duration::from_secs(2));
            if root.join(JOURNAL).is_file() {
                return;
            }
            if let Err(error) = remove_dir(&root) {
                warn!("could not remove the workspace of an update that did not start: {error}");
            }
        });
    }
}

/// Moves `from` to `to`, making `to`'s folders. A move across volumes falls back to a copy and a removal.
pub(crate) fn move_file(from: &Path, to: &Path) -> Result<(), String> {
    if let Some(parent) = to.parent() {
        std::fs::create_dir_all(parent).map_err(|error| error.to_string())?;
    }
    if std::fs::rename(from, to).is_ok() {
        return Ok(());
    }
    std::fs::copy(from, to).map_err(|error| error.to_string())?;
    std::fs::remove_file(from).map_err(|error| error.to_string())
}

/// Removes a file. One that is already gone counts as removed.
fn remove_file(path: &Path) -> Result<(), String> {
    match std::fs::remove_file(path) {
        Ok(()) => Ok(()),
        Err(error) if error.kind() == ErrorKind::NotFound => Ok(()),
        Err(error) => Err(error.to_string()),
    }
}

/// Removes a folder and everything in it. One that is already gone counts as removed.
fn remove_dir(path: &Path) -> Result<(), String> {
    match std::fs::remove_dir_all(path) {
        Ok(()) => Ok(()),
        Err(error) if error.kind() == ErrorKind::NotFound => Ok(()),
        Err(error) => Err(error.to_string()),
    }
}

/// Settles the updates an earlier launch left unfinished, one workspace at a time. `instances` maps each instance the database
/// holds to its folder, and `committed` maps each instance to the token of the update it last committed. A workspace whose
/// journal carries that token committed, so it is finished. Any other is rolled back. A workspace with no journal, or for an
/// instance that no longer exists, is removed. Returns the instances whose update was rolled back.
pub(crate) fn settle(paths: &LauncherPaths, instances: &BTreeMap<String, PathBuf>, committed: &BTreeMap<String, String>) -> Vec<String> {
    let Ok(entries) = std::fs::read_dir(paths.root().join("pack-update")) else {
        return Vec::new();
    };
    let mut rolled_back = Vec::new();
    for entry in entries.flatten() {
        let id = entry.file_name().to_string_lossy().into_owned();
        let root = entry.path();
        let Some(folder) = instances.get(&id) else {
            warn!("removing the workspace of an update to an instance that no longer exists: {id}");
            if let Err(error) = remove_dir(&root) {
                warn!("could not remove {}: {error}", root.display());
            }
            continue;
        };
        let workspace = Workspace::new(root, folder.clone(), paths.pack_base(&id));
        let journal = match workspace.read_journal() {
            Ok(journal) => journal,
            Err(error) => {
                warn!("could not read the journal of the update to {id}, which is kept for a closer look: {error}");
                continue;
            }
        };
        let Some(journal) = journal else {
            // Nothing was written to the instance, so the workspace is only staged files.
            let _ = remove_dir(&workspace.root);
            continue;
        };
        if committed.get(&id) == Some(&journal.token) {
            if let Err(error) = workspace.finish() {
                warn!("could not finish the committed update of {id}: {error}");
            }
        } else {
            match workspace.roll_back() {
                Ok(()) => rolled_back.push(id),
                Err(error) => warn!("could not roll back the update of {id}: {error}"),
            }
        }
    }
    rolled_back
}

#[cfg(test)]
#[path = "../../../../tests/app/install/update/journal.rs"]
mod tests;
