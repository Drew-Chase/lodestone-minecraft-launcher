//! Where the launcher keeps its files. Every folder it writes hangs from one
//! root. Instance folders hold what belongs to one instance: its mods, saves,
//! config and logs. `meta/` holds what instances share, so a library or an asset
//! is downloaded once: the Java runtimes, the version JSON and client jars, the
//! libraries, the asset index and objects, extracted natives and log configs.

use std::path::PathBuf;
use std::sync::OnceLock;

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

    /// The launcher's data root, which holds the folders below.
    pub fn root(&self) -> PathBuf {
        self.root.clone()
    }

    /// The folder saved skins live in: `<root>/skins`.
    pub fn skins(&self) -> PathBuf {
        self.root.join("skins")
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

    /// The folders the Discover page downloads into, kept apart from the instances so that any
    /// instance can import from them: `<root>/shared`.
    pub fn shared(&self) -> PathBuf {
        self.root.join("shared")
    }

    /// A bundled Java runtime, named by its component: `<root>/meta/java/<component>`.
    pub fn java(&self, component: &str) -> PathBuf {
        self.meta().join("java").join(component)
    }

    /// Every version folder: `<root>/meta/versions`.
    pub fn versions(&self) -> PathBuf {
        self.meta().join("versions")
    }

    /// One version's folder, which holds its JSON and its client jar.
    pub fn version_dir(&self, id: &str) -> PathBuf {
        self.versions().join(id)
    }

    /// One version's JSON: `meta/versions/<id>/<id>.json`.
    pub fn version_json(&self, id: &str) -> PathBuf {
        self.version_dir(id).join(format!("{id}.json"))
    }

    /// A Minecraft version's client jar: `meta/versions/<id>/<id>.jar`.
    pub fn client_jar(&self, id: &str) -> PathBuf {
        self.version_dir(id).join(format!("{id}.jar"))
    }

    /// Every library any version needs, laid out as Maven paths: `<root>/meta/libraries`.
    pub fn libraries(&self) -> PathBuf {
        self.meta().join("libraries")
    }

    /// The asset store, shared by every version: `<root>/meta/assets`.
    pub fn assets(&self) -> PathBuf {
        self.meta().join("assets")
    }

    /// The asset indexes, one JSON per index id: `meta/assets/indexes`.
    pub fn asset_indexes(&self) -> PathBuf {
        self.assets().join("indexes")
    }

    /// The asset objects, named by hash: `meta/assets/objects`.
    pub fn asset_objects(&self) -> PathBuf {
        self.assets().join("objects")
    }

    /// The natives a version extracts: `<root>/meta/natives/<id>`.
    pub fn natives(&self, id: &str) -> PathBuf {
        self.meta().join("natives").join(id)
    }

    /// The logging configs the version JSONs name: `<root>/meta/log_configs`.
    pub fn log_configs(&self) -> PathBuf {
        self.meta().join("log_configs")
    }
}

impl Default for LauncherPaths {
    /// `<data dir>/lodestone`, the folder the instance list has always used. A portable launcher uses its own root.
    fn default() -> Self {
        Self::new(data_root())
    }
}

/// The roots a portable launcher keeps its files under. They are set once, before the window opens, when the launcher
/// carries a package.
static PORTABLE_ROOTS: OnceLock<PortableRoots> = OnceLock::new();

struct PortableRoots {
    config: PathBuf,
    data: PathBuf,
}

/// Moves the launcher's folders under the portable roots of the package `id`. A portable launcher then never shares a
/// library, a database or a sign-in with the launcher installed beside it.
pub fn use_portable_roots(id: &str) {
    let config = dirs::config_dir()
        .unwrap_or_else(|| PathBuf::from("."))
        .join("lodestone-portable")
        .join(id);
    let data = dirs::data_dir().unwrap_or_else(std::env::temp_dir).join("lodestone-portable").join(id);
    let _ = PORTABLE_ROOTS.set(PortableRoots { config, data });
}

/// The folder the launcher keeps its settings, its sign-in, its database and the records of its running games in:
/// `<config dir>/lodestone`, or the portable root when the launcher is portable.
pub fn config_dir() -> Option<PathBuf> {
    match PORTABLE_ROOTS.get() {
        Some(roots) => Some(roots.config.clone()),
        None => dirs::config_dir().map(|dir| dir.join("lodestone")),
    }
}

/// The data root: `<data dir>/lodestone`, or the portable root when the launcher is portable.
fn data_root() -> PathBuf {
    match PORTABLE_ROOTS.get() {
        Some(roots) => roots.data.clone(),
        None => dirs::data_dir().unwrap_or_else(std::env::temp_dir).join("lodestone"),
    }
}

#[cfg(test)]
#[path = "../../tests/app/paths.rs"]
mod tests;
