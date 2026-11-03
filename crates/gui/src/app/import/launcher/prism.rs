//! Prism Launcher and MultiMC. Each instance is a folder that holds `mmc-pack.json`, which lists its components by uid and
//! version, and `instance.cfg`, which names it. The game files are in `minecraft\`, or in `.minecraft\` for an instance an
//! older version made.

use std::fs;
use std::path::{Path, PathBuf};

use serde::Deserialize;

use super::{
    Instance, Launcher, Meta, child_folders, folder_name, forge_version, instance, no_game_folder, no_version, read_json, unsupported_loader,
};
use crate::app::pages::library::instance::Loader;

/// The uid of the Minecraft component, and of each loader component this app installs.
const MINECRAFT: &str = "net.minecraft";
const FABRIC: &str = "net.fabric.fabric-loader";
const QUILT: &str = "org.quiltmc.quilt-loader";
const FORGE: &str = "net.minecraftforge";
const NEOFORGE: &str = "net.neoforged";
const LITELOADER: &str = "com.mumfrey.liteloader";

/// The parts of `mmc-pack.json` the import reads.
#[derive(Deserialize)]
struct PackFile {
    components: Vec<Component>,
}

#[derive(Deserialize)]
struct Component {
    uid: String,
    #[serde(default)]
    version: Option<String>,
}

/// The instances folder for `folder`: the folder itself, or its `instances` folder, when one of them holds instances.
pub(super) fn instances_in(folder: &Path) -> Option<PathBuf> {
    [
        folder.to_path_buf(),
        folder.join("instances"),
        folder.join("PrismLauncher").join("instances"),
    ]
    .into_iter()
    .find(|dir| child_folders(dir).iter().any(|instance| instance.join("mmc-pack.json").is_file()))
}

/// The instances in `dir`: each folder that holds an `mmc-pack.json`.
pub(super) fn list(launcher: Launcher, dir: &Path) -> Result<Vec<Instance>, String> {
    Ok(child_folders(dir)
        .into_iter()
        .filter(|folder| folder.join("mmc-pack.json").is_file())
        .map(|folder| {
            let name = folder_name(&folder);
            let source = folder.to_string_lossy().into_owned();
            instance(launcher, source, name, read(&folder))
        })
        .collect())
}

/// Reads the instance in `folder`.
pub(super) fn read(folder: &Path) -> Result<Meta, String> {
    let pack: PackFile = read_json(&folder.join("mmc-pack.json"))?;
    let minecraft = pack
        .components
        .iter()
        .find(|component| component.uid == MINECRAFT)
        .and_then(|component| component.version.clone())
        .ok_or_else(no_version)?;
    let mut loader = Loader::Vanilla;
    let mut loader_version = None;
    for component in &pack.components {
        let Some(version) = component.version.clone() else {
            continue;
        };
        match component.uid.as_str() {
            FABRIC => {
                loader = Loader::Fabric;
                loader_version = Some(version);
            }
            QUILT => {
                loader = Loader::Quilt;
                loader_version = Some(version);
            }
            FORGE => {
                loader = Loader::Forge;
                loader_version = Some(forge_version(&minecraft, &version));
            }
            NEOFORGE => {
                loader = Loader::NeoForge;
                loader_version = Some(version);
            }
            LITELOADER => return Err(unsupported_loader("LiteLoader")),
            _ => {}
        }
    }
    let game_dir = ["minecraft", ".minecraft"]
        .into_iter()
        .map(|name| folder.join(name))
        .find(|dir| dir.is_dir())
        .ok_or_else(no_game_folder)?;
    Ok(Meta {
        name: instance_name(folder),
        minecraft,
        loader,
        loader_version,
        game_dir,
        skip: Vec::new(),
    })
}

/// The name in the instance's `instance.cfg`, or the folder name when the file gives none.
fn instance_name(folder: &Path) -> String {
    let named = fs::read_to_string(folder.join("instance.cfg")).ok().and_then(|text| {
        text.lines()
            .find_map(|line| line.strip_prefix("name=").map(|name| name.trim().to_string()))
    });
    named.filter(|name| !name.is_empty()).unwrap_or_else(|| folder_name(folder))
}

#[cfg(test)]
#[path = "../../../../tests/app/import/launcher/prism.rs"]
mod tests;
