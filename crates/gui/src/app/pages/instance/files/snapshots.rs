//! The mod snapshots an instance keeps in `.lodestone/snapshots/`: each one's name, and what its JSON lists. The Snapshots tab
//! and the rollbacks read them from here.

use std::path::Path;

use lodestone_snapshots::Snapshot;
use lodestone_snapshots::store::{self, Listed};

/// A snapshot the instance folder holds, with what its JSON lists.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SnapshotFile {
    /// The snapshot's name, and the files it is kept in.
    pub listed: Listed,
    /// What the snapshot holds.
    pub snapshot: Snapshot,
}

/// The snapshots in the instance folder `folder`, newest first. A snapshot whose JSON cannot be read is left out and logged,
/// so one damaged file does not hide the rest. Blocks, so it runs on the blocking pool.
pub fn scan(folder: &Path) -> Vec<SnapshotFile> {
    let dir = store::dir(folder);
    let listed = match store::list(&dir) {
        Ok(listed) => listed,
        Err(error) => {
            tracing::warn!("reading the snapshots in {} failed: {error}", dir.display());
            return Vec::new();
        }
    };
    listed
        .into_iter()
        .filter_map(|listed| match store::read(&listed.json) {
            Ok(snapshot) => Some(SnapshotFile { listed, snapshot }),
            Err(error) => {
                tracing::warn!("reading the snapshot {} failed: {error}", listed.json.display());
                None
            }
        })
        .collect()
}

#[cfg(test)]
#[path = "../../../../../tests/app/pages/instance/files/snapshots.rs"]
mod tests;
