//! Where the launcher keeps its files. Every folder it writes hangs from one
//! root. Instance folders hold what belongs to one instance: its mods, saves,
//! config and logs. `meta/` holds what instances share, so a library or an asset
//! is downloaded once: the Java runtimes, the version JSON and client jars, the
//! libraries, the asset index and objects, extracted natives and log configs.

use std::ffi::OsStr;
use std::path::{Path, PathBuf};
use std::sync::OnceLock;

/// The launcher's data root and the folders under it.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct LauncherPaths {
    root: PathBuf,
    /// The instances folder the user chose, when it is not the default one inside the root.
    instances: Option<PathBuf>,
    /// The logs folder the user chose. A chosen folder takes effect at the next start.
    logs: Option<PathBuf>,
    /// The cache folder the user chose. A chosen folder takes effect at the next start.
    cache: Option<PathBuf>,
}

impl LauncherPaths {
    /// The paths under `root`, with the instances kept inside it and the logs and cache in their default folders.
    pub fn new(root: impl Into<PathBuf>) -> Self {
        Self {
            root: root.into(),
            instances: None,
            logs: None,
            cache: None,
        }
    }

    /// The same paths, with the instances kept in `folder`. `None` keeps them inside the root.
    pub fn with_instances(mut self, folder: Option<PathBuf>) -> Self {
        self.instances = folder;
        self
    }

    /// The same paths, with the logs kept in `folder`. `None` keeps them in the default folder.
    pub fn with_logs(mut self, folder: Option<PathBuf>) -> Self {
        self.logs = folder;
        self
    }

    /// The same paths, with the cache kept in `folder`. `None` keeps it in the default folder.
    pub fn with_cache(mut self, folder: Option<PathBuf>) -> Self {
        self.cache = folder;
        self
    }

    /// The same paths under another root. The folders the user chose stay where they are, while the default instances folder
    /// moves with the root.
    pub fn with_root(&self, root: impl Into<PathBuf>) -> Self {
        Self {
            root: root.into(),
            instances: self.instances.clone(),
            logs: self.logs.clone(),
            cache: self.cache.clone(),
        }
    }

    /// The instances folder the user chose, when it is not the default one inside the root.
    pub fn instances_chosen(&self) -> Option<&Path> {
        self.instances.as_deref()
    }

    /// The logs folder the user chose, when it is not the default one.
    pub fn logs_chosen(&self) -> Option<&Path> {
        self.logs.as_deref()
    }

    /// The cache folder the user chose, when it is not the default one.
    pub fn cache_chosen(&self) -> Option<&Path> {
        self.cache.as_deref()
    }

    /// The logs folder the launcher has chosen: the one the user picked, else the default. It is used from the next start.
    pub fn logs(&self) -> PathBuf {
        self.logs.clone().or_else(default_logs_dir).unwrap_or_default()
    }

    /// The cache folder the launcher has chosen: the one the user picked, else the default. It is used from the next start.
    pub fn cache(&self) -> PathBuf {
        self.cache.clone().or_else(default_cache_dir).unwrap_or_default()
    }

    /// The launcher's data root, which holds the folders below.
    pub fn root(&self) -> PathBuf {
        self.root.clone()
    }

    /// The folder saved skins live in: `<root>/skins`.
    pub fn skins(&self) -> PathBuf {
        self.root.join("skins")
    }

    /// Every instance folder: the one the user chose, else `<root>/instances`.
    pub fn instances(&self) -> PathBuf {
        match &self.instances {
            Some(folder) => folder.clone(),
            None => self.root.join("instances"),
        }
    }

    /// One instance's folder, named by its cleaned directory name.
    pub fn instance(&self, directory_name: &str) -> PathBuf {
        self.instances().join(directory_name)
    }

    /// The pristine copies of the text files an installed modpack release shipped, for one instance: `<root>/pack-base/<id>`.
    /// They sit outside the instance folder, so an export and the instance's file watcher never see them.
    pub fn pack_base(&self, instance_id: &str) -> PathBuf {
        self.root.join("pack-base").join(instance_id)
    }

    /// The workspace of one in-place pack update: `<root>/pack-update/<id>`. It holds the journal that lets an interrupted
    /// update roll back, the files the update staged, and the originals it moved aside. It sits outside the instance folder.
    pub fn pack_journal(&self, instance_id: &str) -> PathBuf {
        self.root.join("pack-update").join(instance_id)
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
    /// The default data folder, or the data folder of the fixed root (portable or development) when one applies.
    fn default() -> Self {
        Self::new(resolve_data_root(None))
    }
}

/// The roots a portable launcher keeps its files under. They are set once, before the window opens, when the launcher
/// carries a package.
static PORTABLE_ROOTS: OnceLock<FixedRoots> = OnceLock::new();

/// The roots a development run keeps its files under, from `APP_DIRECTORY` (see `.cargo/config.toml`). They are set once,
/// before the window opens, so a development run never touches the installed launcher's folders.
static DEVELOPMENT_ROOTS: OnceLock<FixedRoots> = OnceLock::new();

/// One root, and the three folders the launcher keeps its files in under it.
struct FixedRoots {
    root: PathBuf,
    config: PathBuf,
    data: PathBuf,
    cache: PathBuf,
}

impl FixedRoots {
    /// The config, data and cache folders under `root`: `config/`, `data/` and `cache/`.
    fn under(root: PathBuf) -> Self {
        Self {
            config: root.join("config"),
            data: root.join("data"),
            cache: root.join("cache"),
            root,
        }
    }
}

/// The fixed root the launcher runs under: a portable launcher's, else a development run's, else none.
fn fixed_roots() -> Option<&'static FixedRoots> {
    PORTABLE_ROOTS.get().or_else(|| DEVELOPMENT_ROOTS.get())
}

/// The logs and cache folders the user chose, read from the preferences once at the start. A chosen folder takes effect at the
/// next start, since the logs open before the window does.
static SAVED_FOLDERS: OnceLock<SavedFolders> = OnceLock::new();

/// The folders the user chose for the logs and the cache. `None` is the default folder.
#[derive(Debug, Default)]
struct SavedFolders {
    logs: Option<PathBuf>,
    cache: Option<PathBuf>,
}

/// Reads the logs and cache folders the user chose. A portable launcher or a development run keeps its fixed folders, so it
/// reads none. Call it after the roots are chosen and before logging starts.
pub fn use_saved_folders() {
    if fixed_roots().is_some() {
        return;
    }
    let saved = crate::app::preferences::Preferences::load();
    let _ = SAVED_FOLDERS.set(SavedFolders {
        logs: saved.logs_folder,
        cache: saved.cache_folder,
    });
}

/// The folder the user chose for one of the saved folders, when there is one.
fn saved_folder(pick: impl Fn(&SavedFolders) -> Option<PathBuf>) -> Option<PathBuf> {
    SAVED_FOLDERS.get().and_then(pick)
}

/// The app bundle that a macOS launcher runs from: `Lodestone.app` for `Lodestone.app/Contents/MacOS/<launcher>`. None for
/// any other file.
pub fn app_bundle(executable: &Path) -> Option<PathBuf> {
    let folder = executable.parent()?;
    let bundle = folder.parent()?.parent()?;
    let inside = cfg!(target_os = "macos") && folder.ends_with("Contents/MacOS") && bundle.extension()? == "app";
    inside.then(|| bundle.to_path_buf())
}

/// The folder a portable launcher keeps all of its files in: `lodestone-portable/<id>` beside the launcher. On macOS the
/// launcher runs from inside its app bundle, and writing into the bundle would break its code signature, so the folder
/// goes beside the bundle instead.
pub fn portable_root(executable: &Path, id: &str) -> PathBuf {
    let beside = match app_bundle(executable) {
        Some(bundle) => bundle.parent().map(Path::to_path_buf),
        None => executable.parent().map(Path::to_path_buf),
    };
    beside.unwrap_or_else(|| PathBuf::from(".")).join("lodestone-portable").join(id)
}

/// Moves the launcher's folders under the portable root of the package `id`, beside the launcher. A portable launcher then
/// never shares a library, a database or a sign-in with the launcher installed beside it, and it keeps nothing outside its
/// own folder.
pub fn use_portable_roots(id: &str, executable: &Path) {
    let _ = PORTABLE_ROOTS.set(FixedRoots::under(portable_root(executable, id)));
}

/// Keeps the launcher's files under the folder `APP_DIRECTORY` names, as a development run does. An unset or blank value
/// changes nothing. Set once, before the window opens.
pub fn use_app_directory(value: Option<&OsStr>) {
    if let Some(root) = app_directory(value) {
        let _ = DEVELOPMENT_ROOTS.set(FixedRoots::under(root));
    }
}

/// The folder `APP_DIRECTORY` names, made absolute. None when the variable is unset or blank.
fn app_directory(value: Option<&OsStr>) -> Option<PathBuf> {
    let value = value.filter(|value| !value.to_string_lossy().trim().is_empty())?;
    let path = PathBuf::from(value);
    Some(std::path::absolute(&path).unwrap_or(path))
}

/// The folder the launcher keeps its settings, its sign-in, its database and the records of its running games in:
/// `<config dir>/lodestone`, or the `config` folder of the portable or development root when one applies.
pub fn config_dir() -> Option<PathBuf> {
    match fixed_roots() {
        Some(roots) => Some(roots.config.clone()),
        None => dirs::config_dir().map(|dir| dir.join("lodestone")),
    }
}

/// The folder the launcher keeps caches in, such as the news feed, downloaded pictures and staged updates:
/// `<cache dir>/lodestone`, or the `cache` folder of the portable or development root when one applies.
pub fn cache_dir() -> Option<PathBuf> {
    saved_folder(|saved| saved.cache.clone()).or_else(default_cache_dir)
}

/// The cache folder without the user's choice: the cache folder of the portable or development root when one applies, else
/// `<cache dir>/lodestone`.
pub fn default_cache_dir() -> Option<PathBuf> {
    match fixed_roots() {
        Some(roots) => Some(roots.cache.clone()),
        None => dirs::cache_dir().map(|dir| dir.join("lodestone")),
    }
}

/// The folder the launcher writes its logs to: `logs/` in the config folder, so a portable launcher's logs go to its own.
pub fn logs_dir() -> Option<PathBuf> {
    saved_folder(|saved| saved.logs.clone()).or_else(default_logs_dir)
}

/// The logs folder without the user's choice: `logs/` in the config folder, so a portable launcher's logs go to its own.
pub fn default_logs_dir() -> Option<PathBuf> {
    config_dir().map(|dir| dir.join("logs"))
}

/// The data folder the launcher keeps unless the user chooses another: `<data dir>/lodestone`, or the `data` folder of the
/// portable or development root when one applies.
pub fn default_data_root() -> PathBuf {
    match fixed_roots() {
        Some(roots) => roots.data.clone(),
        None => dirs::data_dir().unwrap_or_else(std::env::temp_dir).join("lodestone"),
    }
}

/// The data root the launcher works under. A portable launcher or a development run uses its own root, whatever `chosen`
/// holds. Otherwise it uses `chosen`, the folder the user picked on the Settings page, and the default folder when there is
/// none.
pub fn resolve_data_root(chosen: Option<&Path>) -> PathBuf {
    match fixed_roots() {
        Some(roots) => roots.data.clone(),
        None => chosen.map_or_else(default_data_root, Path::to_path_buf),
    }
}

/// Whether the launcher is portable. A portable launcher's data folder is its own, and cannot be moved.
pub fn is_portable() -> bool {
    PORTABLE_ROOTS.get().is_some()
}

/// Whether the data folder is fixed in place: a portable launcher's, or a development run's. A fixed folder cannot be moved.
pub fn data_folder_fixed() -> bool {
    fixed_roots().is_some()
}

/// The folder a development run keeps the launcher's files in, from `APP_DIRECTORY`. None for a portable launcher, whose
/// own root is the one in use.
pub fn development_root() -> Option<PathBuf> {
    if is_portable() {
        return None;
    }
    DEVELOPMENT_ROOTS.get().map(|roots| roots.root.clone())
}

#[cfg(test)]
#[path = "../../tests/app/paths.rs"]
mod tests;
