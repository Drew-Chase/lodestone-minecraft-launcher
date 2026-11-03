//! The official Minecraft launcher. Its profiles live in `launcher_profiles.json`, or `launcher_profiles_microsoft_store.json`
//! for the Store build. A profile the player made is an instance: its game folder is its `gameDir`, or the `.minecraft` folder
//! when it names none. The built-in latest-release and latest-snapshot profiles are not instances.

use std::collections::BTreeMap;
use std::path::{Path, PathBuf};

use serde::Deserialize;

use super::{Instance, Launcher, Meta, forge_version, instance, no_game_folder, no_version, read_json, unrecognised};
use crate::app::pages::library::instance::Loader;

/// The profile files the launcher keeps, the Store build's last.
const PROFILE_FILES: [&str; 2] = ["launcher_profiles.json", "launcher_profiles_microsoft_store.json"];

/// The profile types the launcher creates for itself. They are not instances.
const BUILT_IN: [&str; 2] = ["latest-release", "latest-snapshot"];

/// The parts of a profiles file the import reads.
#[derive(Deserialize)]
struct ProfilesFile {
    #[serde(default)]
    profiles: BTreeMap<String, Profile>,
}

#[derive(Deserialize)]
struct Profile {
    #[serde(default)]
    name: Option<String>,
    #[serde(rename = "type", default)]
    kind: Option<String>,
    #[serde(rename = "lastVersionId", default)]
    last_version_id: Option<String>,
    #[serde(rename = "gameDir", default)]
    game_dir: Option<PathBuf>,
}

/// The `.minecraft` folder for `folder`: the folder itself, or its `.minecraft` folder, when it holds a profiles file.
pub(super) fn instances_in(folder: &Path) -> Option<PathBuf> {
    [folder.to_path_buf(), folder.join(".minecraft")]
        .into_iter()
        .find(|dir| profiles_file(dir).is_some())
}

/// The instances in `dir`: each custom profile, with its source as `<folder>#<profile id>`.
pub(super) fn list(launcher: Launcher, dir: &Path) -> Result<Vec<Instance>, String> {
    let Some(path) = profiles_file(dir) else {
        return Ok(Vec::new());
    };
    let file: ProfilesFile = read_json(&path)?;
    Ok(file
        .profiles
        .iter()
        .filter(|(_, profile)| !BUILT_IN.contains(&profile.kind.as_deref().unwrap_or("")))
        .map(|(id, profile)| {
            let name = profile.name.clone().filter(|name| !name.trim().is_empty()).unwrap_or_else(|| id.clone());
            let source = format!("{}#{id}", dir.to_string_lossy());
            instance(launcher, source, name, read(dir, id))
        })
        .collect())
}

/// Reads the profile `profile_id` from the `.minecraft` folder `folder`.
pub(super) fn read(folder: &Path, profile_id: &str) -> Result<Meta, String> {
    let path = profiles_file(folder).ok_or_else(no_game_folder)?;
    let file: ProfilesFile = read_json(&path)?;
    let profile = file.profiles.get(profile_id).ok_or_else(unrecognised)?;
    let version = profile.last_version_id.as_deref().ok_or_else(no_version)?;
    let (minecraft, loader, loader_version) = parse_version(version)?;
    let game_dir = profile.game_dir.clone().unwrap_or_else(|| folder.to_path_buf());
    if !game_dir.is_dir() {
        return Err(no_game_folder());
    }
    Ok(Meta {
        name: profile
            .name
            .clone()
            .filter(|name| !name.trim().is_empty())
            .unwrap_or_else(|| profile_id.to_string()),
        minecraft,
        loader,
        loader_version,
        game_dir,
        skip: Vec::new(),
    })
}

/// The profiles file in `dir`, the first of the two the launcher keeps that exists.
fn profiles_file(dir: &Path) -> Option<PathBuf> {
    PROFILE_FILES.iter().map(|name| dir.join(name)).find(|path| path.is_file())
}

/// The Minecraft version, loader and loader version a launch version id names. The ids the launcher writes are a bare
/// version (`1.20.1`), `fabric-loader-<loader>-<version>`, `quilt-loader-<loader>-<version>`, `<version>-forge-<build>`, or
/// `neoforge-<build>`. A NeoForge build of `21.1.x` runs on Minecraft `1.21.1`.
fn parse_version(id: &str) -> Result<(String, Loader, Option<String>), String> {
    if let Some(rest) = id.strip_prefix("fabric-loader-") {
        return loader_and_minecraft(rest, Loader::Fabric);
    }
    if let Some(rest) = id.strip_prefix("quilt-loader-") {
        return loader_and_minecraft(rest, Loader::Quilt);
    }
    if let Some((minecraft, build)) = id.split_once("-forge-") {
        return Ok((minecraft.to_string(), Loader::Forge, Some(forge_version(minecraft, build))));
    }
    if let Some(build) = id.strip_prefix("neoforge-") {
        let mut parts = build.split('.');
        let (Some(major), Some(minor)) = (parts.next(), parts.next()) else {
            return Err(unrecognised());
        };
        let minecraft = if minor == "0" {
            format!("1.{major}")
        } else {
            format!("1.{major}.{minor}")
        };
        return Ok((minecraft, Loader::NeoForge, Some(build.to_string())));
    }
    if id.contains('-') {
        return Err(unrecognised());
    }
    Ok((id.to_string(), Loader::Vanilla, None))
}

/// A fabric or quilt id's `<loader version>-<minecraft version>` part.
fn loader_and_minecraft(rest: &str, loader: Loader) -> Result<(String, Loader, Option<String>), String> {
    let (loader_version, minecraft) = rest.rsplit_once('-').ok_or_else(unrecognised)?;
    if loader_version.is_empty() || minecraft.is_empty() {
        return Err(unrecognised());
    }
    Ok((minecraft.to_string(), loader, Some(loader_version.to_string())))
}

#[cfg(test)]
#[path = "../../../../tests/app/import/launcher/minecraft.rs"]
mod tests;
