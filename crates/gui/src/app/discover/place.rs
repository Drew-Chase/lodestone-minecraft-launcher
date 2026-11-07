//! Placing a project from the Discover page into an instance's pack folder, a world's datapacks, or the instance's saves as a
//! world. The release is the one the user picked, or else the newest that supports the instance's Minecraft version. A
//! release the shared folder already holds is not downloaded again. The copies run on the blocking pool.

use std::path::{Path, PathBuf};

use super::{download_version, versions_of};
use crate::app::lang;
use crate::app::pages::discover::{Destination, PlaceOutcome, PlaceRequest};
use crate::app::pages::instance::files::world_archive;
use crate::app::paths::LauncherPaths;
use crate::app::shared_library::{self, AddedContent, SharedRecord};

/// Where the files of a release are copied to.
enum Target {
    /// A folder of the instance or of a world. Packs and datapacks are copied into it as they are.
    Folder(PathBuf),
    /// The instance's saves folder. A world is imported into it as a save of its own.
    Saves(PathBuf),
}

/// Places `request.project` at its destination. `directory` is the instance's folder name in the library.
pub(crate) async fn place_project(request: PlaceRequest, paths: LauncherPaths, directory: String) -> Result<PlaceOutcome, String> {
    let PlaceRequest {
        project,
        minecraft,
        destination,
        shared,
    } = request;
    let releases = versions_of(project.source, project.content, &project.project_id).await?;
    let release = match project.version_id.as_deref() {
        Some(id) => releases.into_iter().find(|release| release.id == id),
        None => releases.into_iter().find(|release| release.minecraft_versions.contains(&minecraft)),
    }
    .ok_or_else(|| lang::t("discover.no_version").to_string())?;

    let shared_folder = shared_library::folder(&paths, project.content);
    let (source, record) = match shared.filter(|record| record.version_id == release.id) {
        Some(record) => (shared_folder.join(&record.path), None),
        None => {
            let folder = shared_library::download_folder(project.source.key(), &project.project_id, &release.id);
            let source = shared_folder.join(&folder);
            download_version(project.source, &release, &source).await?;
            let sizes: Vec<u64> = release.files.iter().filter_map(|file| file.size).collect();
            let record = SharedRecord {
                kind: shared_library::folder_name(project.content).to_string(),
                platform: project.source.key().to_string(),
                project_id: project.project_id.clone(),
                version_id: release.id.clone(),
                name: project.name.clone(),
                path: folder,
                size: (!sizes.is_empty()).then(|| sizes.iter().sum()),
            };
            (source, Some(record))
        }
    };

    let instance_folder = paths.instance(&directory);
    let instance = destination.instance().to_string();
    let (target, scope) = match &destination {
        Destination::Pack { .. } => (
            Target::Folder(instance_folder.join(shared_library::folder_name(project.content))),
            String::new(),
        ),
        Destination::Datapacks { world, .. } => (Target::Folder(instance_folder.join("saves").join(world).join("datapacks")), world.clone()),
        Destination::World { .. } => (Target::Saves(instance_folder.join("saves")), String::new()),
    };
    let file_name = tokio::task::spawn_blocking(move || copy_release(&source, target))
        .await
        .map_err(|error| error.to_string())??;

    Ok(PlaceOutcome {
        added: AddedContent {
            instance,
            scope,
            kind: shared_library::folder_name(project.content).to_string(),
            platform: project.source.key().to_string(),
            project_id: project.project_id,
            version_id: release.id,
            name: project.name,
            file_name,
        },
        shared: record,
    })
}

/// Copies the files of a release from its shared folder into `target`, and returns the name the content has there. A
/// world is the release's zip, imported as a save, whose name is the save's folder.
fn copy_release(source: &Path, target: Target) -> Result<String, String> {
    let mut files: Vec<PathBuf> = std::fs::read_dir(source)
        .map_err(|error| error.to_string())?
        .filter_map(Result::ok)
        .map(|entry| entry.path())
        .collect();
    files.sort();
    match target {
        Target::Folder(folder) => {
            let mut first = None;
            for file in &files {
                let name = shared_library::copy_into(file, &folder)?;
                first.get_or_insert(name);
            }
            first.ok_or_else(|| lang::t("discover.no_version").to_string())
        }
        Target::Saves(saves) => {
            let archive = files
                .iter()
                .find(|file| file.extension().is_some_and(|extension| extension.eq_ignore_ascii_case("zip")))
                .or(files.first())
                .ok_or_else(|| lang::t("discover.no_version").to_string())?;
            world_archive::import(archive, &saves)
        }
    }
}
