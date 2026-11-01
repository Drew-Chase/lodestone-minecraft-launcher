//! Adding a mod to an instance: the release that fits the instance, the required dependencies its `mods/` folder
//! lacks, and the jars written into that folder. Every release is chosen before any file is written, so a dependency
//! with no release that fits leaves the folder as it was. The answer names every jar the mod needs, its own first, as
//! the platform source each came from.

use std::collections::{HashSet, VecDeque};
use std::path::PathBuf;

use lodestone_platforms::{ContentFile, ContentVersion, DependencyKind, VersionDependency};

use crate::app::discover::{download_version, versions_of};
use crate::app::lang;
use crate::app::pages::discover::AddRequest;
use crate::app::shared_library::ModSource;

/// Adds the mod `request` names to the instance it names, writing its jar and the required dependencies the folder
/// `mods` does not already hold. The answer names every jar the mod needs, the mod's own first. A dependency that is
/// already in the folder is named but not written again.
pub(crate) async fn add_to_instance(request: AddRequest, mods: PathBuf) -> Result<Vec<ModSource>, String> {
    let AddRequest { project, instance } = request;
    let loader = instance.loader.to_db();
    let minecraft = instance.mc_version.as_str();

    let releases = versions_of(project.source, project.content, &project.project_id).await?;
    let chosen = match project.version_id.as_deref() {
        Some(id) => releases.into_iter().find(|release| release.id == id),
        None => fitting(releases, loader, minecraft),
    }
    .ok_or_else(|| lang::t("discover.mods.no_release").to_string())?;
    primary(&chosen).ok_or_else(|| lang::t("discover.no_version").to_string())?;

    // Each release the mod needs, with the project it belongs to and whether it is written now.
    let mut jars: Vec<(String, ContentVersion, bool)> = vec![(project.project_id.clone(), chosen.clone(), true)];
    let mut seen = HashSet::from([project.project_id.clone()]);
    let mut queue: VecDeque<VersionDependency> = required(&chosen);
    while let Some(dependency) = queue.pop_front() {
        let Some(dependency_project) = dependency.project_id else {
            continue;
        };
        if !seen.insert(dependency_project.clone()) {
            continue;
        }
        let releases = versions_of(project.source, project.content, &dependency_project).await?;
        let release = fitting(releases, loader, minecraft)
            .ok_or_else(|| lang::t_with("discover.mods.no_dependency", &[("project", dependency_project.as_str())]).to_string())?;
        let present = primary(&release).is_some_and(|file| mods.join(&file.name).is_file());
        if !present {
            queue.extend(required(&release));
        }
        jars.push((dependency_project, release, !present));
    }

    std::fs::create_dir_all(&mods).map_err(|error| error.to_string())?;
    for (_, release, _) in jars.iter().filter(|(_, _, written)| *written) {
        let Some(jar) = primary(release) else {
            continue;
        };
        let mut single = release.clone();
        single.files = vec![jar];
        download_version(project.source, &single, &mods).await?;
    }

    // The mod's own record takes the project's name, which its release does not carry.
    let platform = project.source.key().to_string();
    let own_name = project.name;
    Ok(jars
        .into_iter()
        .enumerate()
        .filter_map(|(index, (project_id, release, _))| {
            let jar = primary(&release)?;
            let name = match index {
                0 => own_name.clone(),
                _ => release.name.clone().unwrap_or_else(|| jar.name.clone()),
            };
            Some(ModSource {
                instance: instance.id.clone(),
                platform: platform.clone(),
                project_id,
                version_id: release.id,
                name,
                file_name: jar.name,
                sha1: jar.sha1,
            })
        })
        .collect())
}

/// The newest release that runs on the loader and the Minecraft version. Platforms list releases newest first.
pub(crate) fn fitting(releases: Vec<ContentVersion>, loader: &str, minecraft: &str) -> Option<ContentVersion> {
    releases.into_iter().find(|release| fits(release, loader, minecraft))
}

/// Whether the release runs on the loader and the Minecraft version.
pub(crate) fn fits(release: &ContentVersion, loader: &str, minecraft: &str) -> bool {
    release.loaders.iter().any(|known| known == loader) && release.minecraft_versions.iter().any(|known| known == minecraft)
}

/// The release's required dependencies that name a project.
fn required(release: &ContentVersion) -> VecDeque<VersionDependency> {
    release
        .dependencies
        .iter()
        .filter(|dependency| dependency.kind == DependencyKind::Required && dependency.project_id.is_some())
        .cloned()
        .collect()
}

/// The release's primary jar, or its first file when none is marked primary.
fn primary(release: &ContentVersion) -> Option<ContentFile> {
    release.files.iter().find(|file| file.primary).or_else(|| release.files.first()).cloned()
}

#[cfg(test)]
#[path = "../../../tests/app/install/mods.rs"]
mod tests;
