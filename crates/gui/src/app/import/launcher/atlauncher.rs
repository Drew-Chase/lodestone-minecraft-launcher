//! ATLauncher. Each instance is a folder that holds `instance.json`: its Minecraft version as `id`, and its name and loader
//! under `launcher`. The game files sit beside that file, in the folder itself.

use std::path::{Path, PathBuf};

use serde::Deserialize;

use super::{Instance, Launcher, Meta, child_folders, folder_name, forge_version, instance, loader_named, no_version, read_json, unsupported_loader};
use crate::app::pages::library::instance::Loader;

/// The file an instance keeps its details in. The import leaves it out of the copy.
const INSTANCE_FILE: &str = "instance.json";

/// The parts of `instance.json` the import reads.
#[derive(Deserialize)]
struct InstanceFile {
    id: String,
    launcher: LauncherInfo,
}

#[derive(Deserialize)]
struct LauncherInfo {
    name: String,
    #[serde(rename = "loaderVersion", default)]
    loader_version: Option<LoaderInfo>,
}

#[derive(Deserialize)]
struct LoaderInfo {
    #[serde(rename = "type", default)]
    kind: Option<String>,
    #[serde(default)]
    version: Option<String>,
}

/// The instances folder for `folder`: its `instances` folder, or the folder itself, when one of them holds instances.
pub(super) fn instances_in(folder: &Path) -> Option<PathBuf> {
    [folder.join("instances"), folder.to_path_buf()]
        .into_iter()
        .find(|dir| child_folders(dir).iter().any(|instance| instance.join(INSTANCE_FILE).is_file()))
}

/// The instances in `dir`: each folder that holds an `instance.json`.
pub(super) fn list(launcher: Launcher, dir: &Path) -> Result<Vec<Instance>, String> {
    Ok(child_folders(dir)
        .into_iter()
        .filter(|folder| folder.join(INSTANCE_FILE).is_file())
        .map(|folder| {
            let name = folder_name(&folder);
            let source = folder.to_string_lossy().into_owned();
            instance(launcher, source, name, read(&folder))
        })
        .collect())
}

/// Reads the instance in `folder`.
pub(super) fn read(folder: &Path) -> Result<Meta, String> {
    let file: InstanceFile = read_json(&folder.join(INSTANCE_FILE))?;
    if file.id.trim().is_empty() {
        return Err(no_version());
    }
    let (loader, loader_version) = match file.launcher.loader_version {
        None => (Loader::Vanilla, None),
        Some(info) => {
            let kind = info.kind.unwrap_or_default();
            let loader = loader_named(&kind).ok_or_else(|| unsupported_loader(&kind))?;
            let version = info.version.filter(|version| !version.trim().is_empty());
            let version = match (loader, version) {
                (Loader::Forge, Some(version)) => Some(forge_version(&file.id, &version)),
                (Loader::Vanilla, _) => None,
                (_, version) => version,
            };
            (loader, version)
        }
    };
    Ok(Meta {
        name: file.launcher.name,
        minecraft: file.id,
        loader,
        loader_version,
        game_dir: folder.to_path_buf(),
        skip: vec![INSTANCE_FILE.to_string()],
    })
}

#[cfg(test)]
#[path = "../../../../tests/app/import/launcher/atlauncher.rs"]
mod tests;
