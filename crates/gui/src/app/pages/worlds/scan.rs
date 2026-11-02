//! The saves of every instance, read for the Worlds page on the blocking pool.

use std::path::PathBuf;

use crate::app::pages::instance::files::{self, WorldFile};
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

#[cfg(test)]
#[path = "../../../../tests/app/pages/worlds/scan.rs"]
mod tests;
