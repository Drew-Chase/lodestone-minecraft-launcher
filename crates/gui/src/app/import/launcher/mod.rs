//! Import from another launcher. The Import dialog's From launcher tab lists the instances of each launcher found on this
//! machine, and copies the checked ones into new instances. A launcher is found in the folders it installs to by default,
//! in the folder its uninstall entry names, or in a folder the user located, which is kept in the preferences. Each launcher
//! keeps its instances in a layout of its own, and each layout has a module here that reads it.

mod atlauncher;
mod curseforge;
mod minecraft;
mod modrinth;
mod prism;
#[cfg(windows)]
mod registry;
mod roots;

use std::fmt::Display;
use std::path::{Path, PathBuf};

use chrono::Local;
use lodestone_platforms::combined_forge;

use super::prepare::Prepared;
use super::record::{ImportFormat, ImportOrigin, ImportRecord};
use crate::app::components::instance_icon;
use crate::app::install::copy;
use crate::app::install::modpack::{Origin, Payload, Unpack};
use crate::app::install::package::random_scene;
use crate::app::install::{IconSource, InstallRequest, new_id};
use crate::app::lang;
use crate::app::pages::library::instance::Loader;
use crate::app::pages::library::new_instance::naming;
use crate::app::paths::LauncherPaths;
use crate::app::preferences::Preferences;

/// The launchers the Import dialog knows, in the order it lists them.
// The variants are the launchers' own names, so `AtLauncher` keeps its name though it ends the way the enum does.
#[allow(clippy::enum_variant_names)]
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum Launcher {
    Prism,
    MultiMc,
    AtLauncher,
    Minecraft,
    CurseForge,
    Modrinth,
}

impl Launcher {
    /// Every launcher, in the order the dialog lists them.
    pub const ALL: [Launcher; 6] = [
        Launcher::Prism,
        Launcher::MultiMc,
        Launcher::AtLauncher,
        Launcher::Minecraft,
        Launcher::CurseForge,
        Launcher::Modrinth,
    ];

    /// The key the launcher's located folder is saved under.
    pub fn key(self) -> &'static str {
        match self {
            Launcher::Prism => "prism",
            Launcher::MultiMc => "multimc",
            Launcher::AtLauncher => "atlauncher",
            Launcher::Minecraft => "minecraft",
            Launcher::CurseForge => "curseforge",
            Launcher::Modrinth => "modrinth",
        }
    }

    /// The name the launcher goes by. Brand names are not translated.
    pub fn name(self) -> &'static str {
        match self {
            Launcher::Prism => "Prism Launcher",
            Launcher::MultiMc => "MultiMC",
            Launcher::AtLauncher => "ATLauncher",
            Launcher::Minecraft => "Minecraft Launcher",
            Launcher::CurseForge => "CurseForge app",
            Launcher::Modrinth => "Modrinth app",
        }
    }

    /// The format an instance of the launcher is recorded under.
    pub fn format(self) -> ImportFormat {
        match self {
            Launcher::Prism => ImportFormat::PrismLauncher,
            Launcher::MultiMc => ImportFormat::MultiMc,
            Launcher::AtLauncher => ImportFormat::AtLauncher,
            Launcher::Minecraft => ImportFormat::MinecraftLauncher,
            Launcher::CurseForge => ImportFormat::CurseForgeApp,
            Launcher::Modrinth => ImportFormat::ModrinthApp,
        }
    }

    /// The launcher a recorded format came from, when the format is a launcher's.
    pub fn from_format(format: ImportFormat) -> Option<Self> {
        Self::ALL.into_iter().find(|launcher| launcher.format() == format)
    }

    /// The launcher stored under `key`, when there is one.
    pub fn from_key(key: &str) -> Option<Self> {
        Self::ALL.into_iter().find(|launcher| launcher.key() == key)
    }

    /// The display names the launcher's uninstall entry may carry, which the registry is searched for.
    fn display_names(self) -> &'static [&'static str] {
        match self {
            Launcher::Prism => &["Prism Launcher"],
            Launcher::MultiMc => &["MultiMC"],
            Launcher::AtLauncher => &["ATLauncher"],
            Launcher::Modrinth => &["Modrinth App"],
            Launcher::Minecraft | Launcher::CurseForge => &[],
        }
    }

    /// The folder that holds this launcher's instances, when `folder` is the launcher's folder or the folder that holds it.
    fn instances_in(self, folder: &Path) -> Option<PathBuf> {
        match self {
            Launcher::Prism | Launcher::MultiMc => prism::instances_in(folder),
            Launcher::AtLauncher => atlauncher::instances_in(folder),
            Launcher::Minecraft => minecraft::instances_in(folder),
            Launcher::CurseForge => curseforge::instances_in(folder),
            Launcher::Modrinth => modrinth::instances_in(folder),
        }
    }

    /// The instances in the folder `instances_in` returned.
    fn list(self, dir: &Path) -> Result<Vec<Instance>, String> {
        match self {
            Launcher::Prism | Launcher::MultiMc => prism::list(self, dir),
            Launcher::AtLauncher => atlauncher::list(self, dir),
            Launcher::Minecraft => minecraft::list(self, dir),
            Launcher::CurseForge => curseforge::list(self, dir),
            Launcher::Modrinth => modrinth::list(self, dir),
        }
    }

    /// Reads the instance at `source`.
    fn read(self, source: &str) -> Result<Meta, String> {
        match self {
            Launcher::Prism | Launcher::MultiMc => prism::read(Path::new(source)),
            Launcher::AtLauncher => atlauncher::read(Path::new(source)),
            Launcher::Minecraft => {
                let (folder, profile) = source.rsplit_once('#').ok_or_else(unrecognised)?;
                minecraft::read(Path::new(folder), profile)
            }
            Launcher::CurseForge => curseforge::read(Path::new(source)),
            Launcher::Modrinth => modrinth::read(Path::new(source)),
        }
    }
}

/// An instance a launcher lists: what the dialog shows, and where the copy reads it from.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Instance {
    /// The launcher the instance belongs to.
    pub launcher: Launcher,
    /// Where the instance is: its folder, or for an official launcher profile `<folder>#<profile id>`. The dialog and the
    /// recents identify the instance by it.
    pub source: String,
    /// The name the launcher gives the instance.
    pub name: String,
    /// The Minecraft version and loader, or why the instance cannot be imported.
    pub summary: Result<String, String>,
}

/// What a launcher's folder held when it was looked at.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Scan {
    /// The launcher the scan is for.
    pub launcher: Launcher,
    /// The folder the launcher's instances are in, when it was found.
    pub folder: Option<PathBuf>,
    /// The instances in that folder.
    pub instances: Vec<Instance>,
    /// Why the folder could not be read, or why the folder saved for the launcher no longer has its instances.
    pub error: Option<String>,
}

/// An instance the user checked: its launcher, and the source the dialog listed it under.
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord)]
pub struct Pick {
    pub launcher: Launcher,
    pub source: String,
}

/// What an instance is, as its launcher records it, and where its game files are.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Meta {
    /// The name the launcher gives the instance.
    pub name: String,
    /// The Minecraft version.
    pub minecraft: String,
    /// The loader the instance runs with.
    pub loader: Loader,
    /// The loader's version, for a modded instance. A Forge version carries the Minecraft version in front.
    pub loader_version: Option<String>,
    /// The folder the game files are in. The copy reads from it.
    pub game_dir: PathBuf,
    /// The top-level names of `game_dir` that the copy leaves out, beyond the names every copy skips.
    pub skip: Vec<String>,
}

/// Scans every launcher on this machine. Each one is looked for in its saved folder first, then its default folders, then the
/// folders its uninstall entry names. Runs on the blocking pool.
pub fn scan_all() -> Vec<Scan> {
    let saved = Preferences::load().launcher_folders;
    Launcher::ALL
        .into_iter()
        .map(|launcher| scan(launcher, saved.get(launcher.key()).cloned()))
        .collect()
}

/// Checks a folder the user picked for `launcher`. When it holds the launcher's instances, the folder is saved and listed.
pub fn locate(launcher: Launcher, folder: PathBuf) -> Result<Scan, String> {
    let Some(dir) = launcher.instances_in(&folder) else {
        return Err(lang::t_with("import.launcher.not_in_folder", &[("name", launcher.name())]).to_string());
    };
    let mut preferences = Preferences::load();
    preferences.launcher_folders.insert(launcher.key().to_string(), folder.clone());
    preferences.store()?;
    Ok(listed(launcher, Some(folder), &dir))
}

/// Reads the checked instances and makes the install of each, with the record the dialog lists it under. Runs on the blocking
/// pool. An instance that cannot be read fails the whole import, with the reason.
pub fn prepare(picks: &[Pick], paths: &LauncherPaths, taken: &[String]) -> Result<Vec<Prepared>, String> {
    let mut taken = taken.to_vec();
    let mut prepared = Vec::with_capacity(picks.len());
    for pick in picks {
        let meta = pick.launcher.read(&pick.source)?;
        let planned = copy::plan(&meta.game_dir, &meta.skip).map_err(unreadable)?;
        let directory_name = naming::unique_directory_name(&paths.instances(), &naming::clean_directory_name(&meta.name), &taken);
        taken.push(directory_name.clone());
        prepared.push(Prepared {
            request: request(paths, &meta, directory_name),
            record: ImportRecord {
                name: meta.name,
                format: pick.launcher.format(),
                origin: ImportOrigin::Launcher,
                source: pick.source.clone(),
                bytes: copy::total_bytes(&planned),
                imported_at: Local::now(),
            },
        });
    }
    Ok(prepared)
}

/// Looks for `launcher` in the saved folder, the default folders, and the folders its uninstall entry names.
fn scan(launcher: Launcher, saved: Option<PathBuf>) -> Scan {
    let mut error = None;
    if let Some(folder) = saved {
        match launcher.instances_in(&folder) {
            Some(dir) => return listed(launcher, Some(folder), &dir),
            None => {
                error = Some(lang::t_with("import.launcher.saved_missing", &[("name", launcher.name())]).to_string());
            }
        }
    }
    let candidates = roots::default_folders(launcher).into_iter().chain(registry_folders(launcher));
    for folder in candidates {
        if let Some(dir) = launcher.instances_in(&folder) {
            return listed(launcher, Some(folder), &dir);
        }
    }
    Scan {
        launcher,
        folder: None,
        instances: Vec::new(),
        error,
    }
}

/// The scan of a folder that holds the launcher's instances.
fn listed(launcher: Launcher, folder: Option<PathBuf>, dir: &Path) -> Scan {
    match launcher.list(dir) {
        Ok(instances) => Scan {
            launcher,
            folder,
            instances,
            error: None,
        },
        Err(error) => Scan {
            launcher,
            folder,
            instances: Vec::new(),
            error: Some(error),
        },
    }
}

/// The install folders the uninstall entries of `launcher` name. Only Windows keeps such entries.
#[cfg(windows)]
fn registry_folders(launcher: Launcher) -> Vec<PathBuf> {
    registry::install_locations(launcher.display_names())
}

/// The install folders the uninstall entries of `launcher` name. Only Windows keeps such entries.
#[cfg(not(windows))]
fn registry_folders(_launcher: Launcher) -> Vec<PathBuf> {
    Vec::new()
}

/// The install for an instance: a new instance under a fresh id, whose game folder the install copies in.
fn request(paths: &LauncherPaths, meta: &Meta, directory_name: String) -> InstallRequest {
    let (background, symbol) = instance_icon::random();
    let (biome, art_seed) = random_scene();
    InstallRequest {
        paths: paths.clone(),
        name: meta.name.clone(),
        directory_name,
        mc_version: meta.minecraft.clone(),
        loader: meta.loader,
        loader_version: meta.loader_version.clone(),
        java_component: None,
        icon: IconSource::Composed {
            background: background.to_string(),
            symbol: symbol.to_string(),
        },
        biome,
        art_seed,
        banner: None,
        id: new_id(),
        source_url: None,
        source_platform: None,
        source_project_id: None,
        modpack: Some(Payload {
            files: Vec::new(),
            unpacks: vec![Unpack {
                origin: Origin::Folder {
                    root: meta.game_dir.clone(),
                    skip: meta.skip.clone(),
                },
                folder: String::new(),
                target: String::new(),
                skip: Vec::new(),
            }],
            sources: Vec::new(),
        }),
        settings: Default::default(),
    }
}

/// The dialog's row for an instance, from what its launcher's metadata read as.
fn instance(launcher: Launcher, source: String, folder_name: String, read: Result<Meta, String>) -> Instance {
    match read {
        Ok(meta) => Instance {
            launcher,
            source,
            summary: Ok(summary(&meta)),
            name: meta.name,
        },
        Err(reason) => Instance {
            launcher,
            source,
            name: folder_name,
            summary: Err(reason),
        },
    }
}

/// The Minecraft version and loader a row shows, such as `1.21.1 · NEOFORGE 21.1.251`.
fn summary(meta: &Meta) -> String {
    match &meta.loader_version {
        Some(version) if meta.loader != Loader::Vanilla => format!("{} · {} {version}", meta.minecraft, meta.loader.label()),
        _ => format!("{} · {}", meta.minecraft, meta.loader.label()),
    }
}

/// The loader a launcher names, when this app can install it. The names are the lower-case loader names launchers use.
fn loader_named(name: &str) -> Option<Loader> {
    match name.to_ascii_lowercase().as_str() {
        "" | "vanilla" => Some(Loader::Vanilla),
        "fabric" => Some(Loader::Fabric),
        "quilt" => Some(Loader::Quilt),
        "forge" => Some(Loader::Forge),
        "neoforge" => Some(Loader::NeoForge),
        _ => None,
    }
}

/// A Forge version in the form the install takes: the Minecraft version in front of the Forge build.
fn forge_version(minecraft: &str, forge: &str) -> String {
    combined_forge(minecraft, forge)
}

/// The reason an instance cannot be imported: its loader is one this app cannot install.
fn unsupported_loader(name: &str) -> String {
    lang::t_with("import.launcher.unsupported_loader", &[("loader", name)]).to_string()
}

/// The reason an instance cannot be imported: its Minecraft version is not recorded.
fn no_version() -> String {
    lang::t("import.launcher.no_version").to_string()
}

/// The reason an instance cannot be imported: its game folder is not there.
fn no_game_folder() -> String {
    lang::t("import.launcher.no_game_folder").to_string()
}

/// The reason an instance cannot be imported: its version is in a form this app does not read.
fn unrecognised() -> String {
    lang::t("import.launcher.unrecognised").to_string()
}

/// The reason a folder could not be read.
fn unreadable(error: impl Display) -> String {
    lang::t_with("import.launcher.unreadable", &[("error", error.to_string().as_str())]).to_string()
}

/// The folders directly inside `dir`, in name order. A folder that cannot be read is taken to be empty.
pub(super) fn child_folders(dir: &Path) -> Vec<PathBuf> {
    let Ok(entries) = std::fs::read_dir(dir) else {
        return Vec::new();
    };
    let mut folders: Vec<PathBuf> = entries.flatten().map(|entry| entry.path()).filter(|path| path.is_dir()).collect();
    folders.sort();
    folders
}

/// The last component of a folder, which is the name a launcher gives an instance it does not name.
pub(super) fn folder_name(folder: &Path) -> String {
    folder
        .file_name()
        .map_or_else(|| folder.to_string_lossy().into_owned(), |name| name.to_string_lossy().into_owned())
}

/// Reads and parses a JSON file. A file that is missing or malformed gives the reason.
pub(super) fn read_json<T: serde::de::DeserializeOwned>(path: &Path) -> Result<T, String> {
    let text = std::fs::read_to_string(path).map_err(unreadable)?;
    serde_json::from_str(&text).map_err(unreadable)
}

#[cfg(test)]
#[path = "../../../../tests/app/import/launcher/mod.rs"]
mod tests;
