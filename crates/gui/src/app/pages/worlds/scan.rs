//! The saves and backups of every instance, read for the Worlds page on the blocking pool.

use std::path::PathBuf;

use crate::app::pages::instance::files::{self, BackupFile, WorldFile};
use crate::app::perf;

/// An instance whose saves the page lists.
#[derive(Debug, Clone)]
pub struct Source {
    pub id: String,
    pub name: String,
    pub mc_version: String,
    /// The instance's folder, which holds `saves/`.
    pub folder: PathBuf,
}

/// One save on the Worlds page, and the instance it belongs to.
#[derive(Debug, Clone)]
pub struct WorldEntry {
    pub instance_id: String,
    pub instance_name: String,
    pub mc_version: String,
    pub world: WorldFile,
}

impl WorldEntry {
    /// Whether the save's name, its seed or its instance's name contains `query`, which is lower case.
    pub fn matches(&self, query: &str) -> bool {
        self.world.matches(query) || self.instance_name.to_lowercase().contains(query)
    }

    /// The line under the save's name: the instance, then the version the save was last written by, or the
    /// instance's own version when the save names none.
    pub fn subtitle(&self) -> String {
        let version = self.world.version.as_deref().unwrap_or(self.mc_version.as_str());
        format!("{} · {}", self.instance_name, version)
    }
}

/// Every save of every source, grouped by source in the order the sources come, and each source's own order.
pub fn scan_all(sources: &[Source]) -> Vec<WorldEntry> {
    let _scope = perf::scope("io.scan_all_worlds");
    sources
        .iter()
        .flat_map(|source| {
            files::scan_worlds(&source.folder).into_iter().map(move |world| WorldEntry {
                instance_id: source.id.clone(),
                instance_name: source.name.clone(),
                mc_version: source.mc_version.clone(),
                world,
            })
        })
        .collect()
}

/// One backup on the Worlds page, and the instance it belongs to.
#[derive(Debug, Clone)]
pub struct BackupEntry {
    pub instance_id: String,
    pub instance_name: String,
    pub backup: BackupFile,
}

impl BackupEntry {
    /// Whether the save's folder name or its instance's name contains `query`, which is lower case.
    pub fn matches(&self, query: &str) -> bool {
        query.is_empty() || self.backup.folder.to_lowercase().contains(query) || self.instance_name.to_lowercase().contains(query)
    }
}

/// Every backup of every source, newest first across the instances.
pub fn scan_backups(sources: &[Source]) -> Vec<BackupEntry> {
    let mut backups: Vec<BackupEntry> = sources
        .iter()
        .flat_map(|source| {
            files::scan_backups(&source.folder).into_iter().map(move |backup| BackupEntry {
                instance_id: source.id.clone(),
                instance_name: source.name.clone(),
                backup,
            })
        })
        .collect();
    backups.sort_by_key(|entry| std::cmp::Reverse(entry.backup.taken));
    backups
}

/// What the Worlds page lists: the saves and the backups of every source, read in one pass.
#[derive(Debug, Clone, Default)]
pub struct Scan {
    pub worlds: Vec<WorldEntry>,
    pub backups: Vec<BackupEntry>,
}

impl Scan {
    /// Reads the saves and the backups of every source.
    pub fn read(sources: &[Source]) -> Self {
        Self {
            worlds: scan_all(sources),
            backups: scan_backups(sources),
        }
    }
}

#[cfg(test)]
#[path = "../../../../tests/app/pages/worlds/scan.rs"]
mod tests;
