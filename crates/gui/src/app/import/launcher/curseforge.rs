//! The CurseForge app. Each instance is a folder under `Instances` that holds `minecraftinstance.json`: its `name`, its
//! Minecraft version as `gameVersion`, and its mod loader as `baseModLoader.name`, such as `neoforge-21.1.251`. The game files
//! sit at the root of the folder, beside that file.

use std::path::{Path, PathBuf};

use serde::Deserialize;

use super::{
    Instance, Launcher, Meta, child_folders, folder_name, forge_version, instance, loader_named, no_version, read_json, unrecognised,
    unsupported_loader,
};
use crate::app::pages::library::instance::Loader;

/// The file an instance keeps its details in.
const METADATA: &str = "minecraftinstance.json";

/// The files and folders the CurseForge app keeps for itself at the root of an instance. None of them is copied.
const SKIP: [&str; 5] = ["minecraftinstance.json", ".curseclient", "manifest.json", "modlist.html", "profileImage"];

/// The parts of `minecraftinstance.json` the import reads.
#[derive(Deserialize)]
struct Metadata {
    #[serde(default)]
    name: Option<String>,
    #[serde(rename = "gameVersion", default)]
    game_version: Option<String>,
    #[serde(rename = "isVanilla", default)]
    is_vanilla: bool,
    #[serde(rename = "baseModLoader", default)]
    base_mod_loader: Option<BaseModLoader>,
}

#[derive(Deserialize)]
struct BaseModLoader {
    #[serde(default)]
    name: Option<String>,
    #[serde(rename = "minecraftVersion", default)]
    minecraft_version: Option<String>,
}

/// The instances folder for `folder`: a folder named `Instances` in it, or the folder itself, when one of them holds instances.
pub(super) fn instances_in(folder: &Path) -> Option<PathBuf> {
    [
        folder.join("Instances"),
        folder.join("minecraft").join("Instances"),
        folder.join("curseforge").join("minecraft").join("Instances"),
        folder.to_path_buf(),
    ]
    .into_iter()
    .find(|dir| child_folders(dir).iter().any(|instance| instance.join(METADATA).is_file()))
}

/// The instances in `dir`: each folder that holds a `minecraftinstance.json`.
pub(super) fn list(launcher: Launcher, dir: &Path) -> Result<Vec<Instance>, String> {
    Ok(child_folders(dir)
        .into_iter()
        .filter(|folder| folder.join(METADATA).is_file())
        .map(|folder| {
            let name = folder_name(&folder);
            let source = folder.to_string_lossy().into_owned();
            instance(launcher, source, name, read(&folder))
        })
        .collect())
}

/// Reads the instance in `folder`.
pub(super) fn read(folder: &Path) -> Result<Meta, String> {
    let metadata: Metadata = read_json(&folder.join(METADATA))?;
    let minecraft = metadata
        .game_version
        .clone()
        .or_else(|| metadata.base_mod_loader.as_ref().and_then(|base| base.minecraft_version.clone()))
        .filter(|version| !version.trim().is_empty())
        .ok_or_else(no_version)?;
    let base_name = metadata
        .base_mod_loader
        .as_ref()
        .and_then(|base| base.name.clone())
        .filter(|name| !name.trim().is_empty());
    let (loader, loader_version) = match base_name {
        Some(id) if !metadata.is_vanilla => {
            let (kind, version) = id.split_once('-').unwrap_or((id.as_str(), ""));
            let loader = loader_named(kind).ok_or_else(|| unsupported_loader(kind))?;
            let version = (!version.is_empty()).then(|| version.to_string());
            match (loader, version) {
                (Loader::Vanilla, _) => (Loader::Vanilla, None),
                (Loader::Forge, Some(build)) => (Loader::Forge, Some(forge_version(&minecraft, &build))),
                (_, version) => (loader, version),
            }
        }
        _ => (Loader::Vanilla, None),
    };
    if loader != Loader::Vanilla && loader_version.is_none() {
        return Err(unrecognised());
    }
    Ok(Meta {
        name: metadata
            .name
            .filter(|name| !name.trim().is_empty())
            .unwrap_or_else(|| folder_name(folder)),
        minecraft,
        loader,
        loader_version,
        game_dir: folder.to_path_buf(),
        skip: SKIP.iter().map(|name| name.to_string()).collect(),
    })
}

#[cfg(test)]
#[path = "../../../../tests/app/import/launcher/curseforge.rs"]
mod tests;
