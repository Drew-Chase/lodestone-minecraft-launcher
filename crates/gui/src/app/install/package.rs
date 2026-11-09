//! A Lodestone package as an install. The portable launcher installs the package it carries this way, and the Import
//! dialog installs a package from a file. The package's linked files become the install's downloads, and each mod
//! becomes a record of where it came from. Its bundled files are added by the install's unpack stage (see `modpack`).

use std::path::Path;

use lodestone_export::format::lodestone::Manifest;
use lodestone_platforms::ModpackFile;

use super::modpack::{Origin, Payload, Unpack};
use super::{IconSource, InstallRequest};
use crate::app::components::instance_icon;
use crate::app::mod_sources::{pack_file, pack_record, stem};
use crate::app::pages::discover::Source;
use crate::app::pages::library::instance::{Biome, Loader};
use crate::app::pages::library::new_instance::naming;
use crate::app::paths::LauncherPaths;
use crate::app::shared_library::ModSource;

/// The biomes the instance's scene art can take. A new instance picks one at random.
const BIOMES: [Biome; 8] = [
    Biome::End,
    Biome::Forest,
    Biome::Cherry,
    Biome::Ocean,
    Biome::Desert,
    Biome::Snow,
    Biome::Nether,
    Biome::Mushroom,
];

/// The package's linked files as an install takes them. A Modrinth file comes with its download; a CurseForge file
/// comes by its ids, and its download is resolved before the install. Each mod, and each pack in a resource or shader
/// folder, also becomes a record of where it came from, which the install keeps for the instance. When `package` is
/// given, the install lays its bundled files over the instance; without it, the caller adds them itself.
pub(crate) fn payload(manifest: &Manifest, package: Option<&Path>) -> Payload {
    let mut files = Vec::new();
    let mut sources = Vec::new();
    let mut packs = Vec::new();
    for record in manifest.files.iter().filter(|record| !record.bundled) {
        let file_name = record.path.strip_prefix("mods/").map(str::to_string);
        let pack = pack_file(&record.path);
        if let Some(link) = &record.links.modrinth {
            files.push(ModpackFile {
                path: record.path.clone(),
                url: link.download_url.clone(),
                sha1: Some(record.sha1.clone()),
                size: Some(record.size),
                project_id: None,
                file_id: None,
            });
            if let Some(file_name) = file_name {
                sources.push(ModSource {
                    instance: String::new(),
                    platform: Source::Modrinth.key().to_string(),
                    project_id: link.project.clone(),
                    version_id: link.version.clone(),
                    name: stem(&file_name),
                    file_name,
                    sha1: Some(record.sha1.clone()),
                });
            } else if let Some((folder, name)) = pack {
                packs.push(pack_record(folder, Source::Modrinth, &link.project, &link.version, &stem(name), name));
            }
        } else if let Some(link) = &record.links.curseforge {
            files.push(ModpackFile {
                path: record.path.clone(),
                url: String::new(),
                sha1: Some(record.sha1.clone()),
                size: Some(record.size),
                project_id: Some(link.project.clone()),
                file_id: Some(link.file.clone()),
            });
            if let Some(file_name) = file_name {
                sources.push(ModSource {
                    instance: String::new(),
                    platform: Source::CurseForge.key().to_string(),
                    project_id: link.project.clone(),
                    version_id: link.file.clone(),
                    name: stem(&file_name),
                    file_name,
                    sha1: Some(record.sha1.clone()),
                });
            } else if let Some((folder, name)) = pack {
                packs.push(pack_record(folder, Source::CurseForge, &link.project, &link.file, &stem(name), name));
            }
        }
    }
    let unpacks: Vec<Unpack> = package
        .map(|path| Unpack {
            origin: Origin::Package(path.to_path_buf()),
            folder: String::new(),
            target: String::new(),
            skip: Vec::new(),
        })
        .into_iter()
        .collect();
    Payload {
        files,
        unpacks,
        sources,
        packs,
    }
}

/// A scene picked at random, for an instance whose modpack has no page to take one from.
pub(crate) fn random_scene() -> (Biome, u64) {
    (BIOMES[fastrand::usize(..BIOMES.len())], fastrand::u64(..))
}

/// The request that installs the package's instance under `id`. The folder is named after the package and kept clear
/// of `taken`, the folders queued installs reserve. A random icon and scene stand in for the ones a new instance would
/// pick.
pub(crate) fn request(paths: &LauncherPaths, manifest: &Manifest, payload: Payload, id: String, taken: &[String]) -> InstallRequest {
    let (background, symbol) = instance_icon::random();
    let base = naming::clean_directory_name(&manifest.name);
    let (biome, art_seed) = random_scene();
    InstallRequest {
        paths: paths.clone(),
        name: manifest.name.clone(),
        directory_name: naming::unique_directory_name(&paths.instances(), &base, taken),
        mc_version: manifest.minecraft.clone(),
        loader: Loader::from_db(&manifest.loader.id),
        loader_version: manifest.loader.version.clone(),
        java_component: None,
        icon: IconSource::Composed {
            background: background.to_string(),
            symbol: symbol.to_string(),
        },
        biome,
        art_seed,
        banner: None,
        id,
        source_url: None,
        source_platform: None,
        source_project_id: None,
        source_version: None,
        source_version_id: None,
        modpack: Some(payload),
        settings: Default::default(),
    }
}

#[cfg(test)]
#[path = "../../../tests/app/install/package.rs"]
mod tests;
