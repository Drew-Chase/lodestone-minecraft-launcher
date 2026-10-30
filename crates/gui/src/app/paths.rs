//! Where the launcher keeps its files. Every folder it writes hangs from one
//! root, so the instance list, the shared Java runtimes and the shared asset
//! indexes always move together.

use std::path::PathBuf;

/// The launcher's data root and the folders under it.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct LauncherPaths {
    root: PathBuf,
}

impl LauncherPaths {
    /// The paths under `root`.
    pub fn new(root: impl Into<PathBuf>) -> Self {
        Self { root: root.into() }
    }

    /// Every instance folder: `<root>/instances`.
    pub fn instances(&self) -> PathBuf {
        self.root.join("instances")
    }

    /// One instance's folder, named by its cleaned directory name.
    pub fn instance(&self, directory_name: &str) -> PathBuf {
        self.instances().join(directory_name)
    }

    /// Downloads shared between instances: `<root>/meta`.
    pub fn meta(&self) -> PathBuf {
        self.root.join("meta")
    }

    /// A bundled Java runtime, named by its component: `<root>/meta/java/<component>`.
    pub fn java(&self, component: &str) -> PathBuf {
        self.meta().join("java").join(component)
    }

    /// The asset index and objects of one Minecraft version: `<root>/meta/assets/<version>`.
    pub fn assets(&self, mc_version: &str) -> PathBuf {
        self.meta().join("assets").join(mc_version)
    }
}

impl Default for LauncherPaths {
    /// `<data dir>/lodestone`, the folder the instance list has always used.
    fn default() -> Self {
        Self::new(dirs::data_dir().unwrap_or_else(std::env::temp_dir).join("lodestone"))
    }
}

#[cfg(test)]
#[path = "../../tests/app/paths.rs"]
mod tests;
