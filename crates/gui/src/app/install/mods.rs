//! Adding a mod to an instance: the release that fits the instance, the required dependencies its `mods/` folder
//! lacks, and the jars written into that folder. Every release is chosen before any file is written, so a dependency
//! with no release that fits leaves the folder as it was.

use std::collections::{HashSet, VecDeque};
use std::path::PathBuf;

use lodestone_platforms::{ContentFile, ContentVersion, DependencyKind, VersionDependency};

use crate::app::discover::{download_version, versions_of};
use crate::app::lang;
use crate::app::pages::discover::AddRequest;
use crate::app::shared_library::InstanceContent;

/// Adds the mod `request` names to the instance it names, writing its jar and the required dependencies the folder
/// `mods` does not already hold. The answer names the jar the mod was written as.
pub(crate) async fn add_to_instance(request: AddRequest, mods: PathBuf) -> Result<InstanceContent, String> {
    let AddRequest { project, instance } = request;
    let loader = instance.loader.to_db();
    let minecraft = instance.mc_version.as_str();

    let releases = versions_of(project.source, project.content, &project.project_id).await?;
    let chosen = match project.version_id.as_deref() {
        Some(id) => releases.into_iter().find(|release| release.id == id),
        None => fitting(releases, loader, minecraft),
    }
    .ok_or_else(|| lang::t("discover.mods.no_release").to_string())?;
    let file = primary(&chosen).ok_or_else(|| lang::t("discover.no_version").to_string())?;

    let mut bundle = vec![chosen.clone()];
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
        if primary(&release).is_some_and(|file| mods.join(&file.name).is_file()) {
            continue;
        }
        queue.extend(required(&release));
        bundle.push(release);
    }

    std::fs::create_dir_all(&mods).map_err(|error| error.to_string())?;
    for release in &bundle {
        let Some(jar) = primary(release) else {
            continue;
        };
        let mut single = release.clone();
        single.files = vec![jar];
        download_version(project.source, &single, &mods).await?;
    }
    Ok(InstanceContent {
        instance: instance.id,
        platform: project.source.key().to_string(),
        project_id: project.project_id,
        version_id: chosen.id,
        name: project.name,
        file_name: file.name,
    })
}

/// The newest release that runs on the loader and the Minecraft version. Platforms list releases newest first.
fn fitting(releases: Vec<ContentVersion>, loader: &str, minecraft: &str) -> Option<ContentVersion> {
    releases
        .into_iter()
        .find(|release| release.loaders.iter().any(|known| known == loader) && release.minecraft_versions.iter().any(|known| known == minecraft))
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
