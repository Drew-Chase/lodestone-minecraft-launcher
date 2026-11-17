//! Adding a mod to an instance: the release that fits the instance, the required dependencies its `mods/` folder
//! lacks, and the jars written into that folder. Every release is chosen, and every dependency is downloaded and read,
//! before any jar lands in `mods/`, so a dependency with no release that fits, or a download that fails, leaves the
//! folder as it was. A dependency is recognised by the mod id its jar declares, so a copy the folder holds from another
//! platform is not written again. The answer names every jar the mod needs, its own first, as the platform source each
//! came from.

use std::collections::{HashSet, VecDeque};
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicUsize, Ordering};

use lodestone_fs::move_file;
use lodestone_platforms::{ContentFile, ContentVersion, DependencyKind, VersionDependency};

use crate::app::discover::{download_version, versions_of};
use crate::app::install::ensure;
use crate::app::lang;
use crate::app::pages::discover::{AddRequest, Source};
use crate::app::pages::instance::files::mods as jar_files;
use crate::app::paths;
use crate::app::shared_library::ModSource;

/// Adds the mod `request` names to the instance it names, writing its jar and the required dependencies the folder `mods`
/// does not already hold. A dependency is held when the folder has its file name, or a jar with its mod id, switched on or
/// off and from any platform. The answer names every jar the mod needs, the mod's own first. A dependency held by its file
/// name is named as it always was; one held only by its mod id is left out, so the copy already there keeps its record.
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
    let own = primary(&chosen).ok_or_else(|| lang::t("discover.no_version").to_string())?;

    std::fs::create_dir_all(&mods).map_err(|error| error.to_string())?;
    // Adds to one folder take turns, so two of them cannot both find a dependency missing and both write it.
    let _folder = ensure::lock(format!("mods:{}", mods.display())).await;
    let staging = Staging::new()?;
    let mut taken = blocking({
        let mods = mods.clone();
        move || jar_files::ids(&mods)
    })
    .await?;

    // The mod's own jar comes first, and it is written whatever the folder holds, as it always was. Its id joins the set,
    // so no dependency is written beside a second copy of the mod.
    let own_staged = stage(project.source, &chosen, &own, staging.dir(0)).await?;
    if let Some(id) = declared(own_staged.clone()).await? {
        taken.insert(id);
    }
    let mut jars = vec![Taken {
        project_id: project.project_id.clone(),
        release: chosen.clone(),
        file: own,
        plan: Plan::Write(own_staged),
    }];

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
        let Some(file) = primary(&release) else {
            // A release with no file writes nothing, but the requirements it names are still walked, as they always were.
            queue.extend(required(&release));
            continue;
        };
        if held_by_name(&mods, &file.name) {
            jars.push(Taken {
                project_id: dependency_project,
                release,
                file,
                plan: Plan::Named,
            });
            continue;
        }
        let staged = stage(project.source, &release, &file, staging.dir(jars.len())).await?;
        let id = declared(staged.clone()).await?;
        let plan = if spoken_for(id.as_deref(), &taken) {
            Plan::Held
        } else {
            if let Some(id) = id {
                taken.insert(id);
            }
            queue.extend(required(&release));
            Plan::Write(staged)
        };
        jars.push(Taken {
            project_id: dependency_project,
            release,
            file,
            plan,
        });
    }

    // The dependencies move first and the mod's own jar last, so a move that fails cannot overwrite the mod's current jar.
    // The own jar is first in `jars` and always written, so it is the first move, and rotating it to the end moves it last.
    let mut moves: Vec<(PathBuf, PathBuf)> = jars
        .iter()
        .filter_map(|jar| match &jar.plan {
            Plan::Write(staged) => Some((staged.clone(), mods.join(&jar.file.name))),
            Plan::Named | Plan::Held => None,
        })
        .collect();
    moves.rotate_left(1);
    blocking(move || move_into(&moves)).await??;

    // The mod's own record takes the project's name, which its release does not carry.
    let platform = project.source.key().to_string();
    let own_name = project.name;
    Ok(jars
        .into_iter()
        .enumerate()
        .filter_map(|(index, jar)| {
            if matches!(jar.plan, Plan::Held) {
                return None;
            }
            let name = match index {
                0 => own_name.clone(),
                _ => jar.release.name.clone().unwrap_or_else(|| jar.file.name.clone()),
            };
            Some(ModSource {
                instance: instance.id.clone(),
                platform: platform.clone(),
                project_id: jar.project_id,
                version_id: jar.release.id,
                name,
                file_name: jar.file.name,
                sha1: jar.file.sha1,
            })
        })
        .collect())
}

/// What one install does with a jar the mod needs.
enum Plan {
    /// The jar is written into `mods/` from its staged download.
    Write(PathBuf),
    /// The folder already holds the file under its name, so nothing is written. The record is written as it always was.
    Named,
    /// A jar with the same mod id is in the folder or was planned by this install, so nothing is written or recorded.
    Held,
}

/// One jar the mod needs, the mod's own first, with what the install does with it.
struct Taken {
    project_id: String,
    release: ContentVersion,
    file: ContentFile,
    plan: Plan,
}

/// Downloads `file`, the primary jar of `release`, into `dir`, and returns the path it landed at. Only that jar is fetched.
async fn stage(source: Source, release: &ContentVersion, file: &ContentFile, dir: PathBuf) -> Result<PathBuf, String> {
    let mut single = release.clone();
    single.files = vec![file.clone()];
    download_version(source, &single, &dir)
        .await?
        .into_iter()
        .next()
        .ok_or_else(|| lang::t("discover.no_version").to_string())
}

/// The mod id a staged jar declares, read on the blocking pool.
async fn declared(jar: PathBuf) -> Result<Option<String>, String> {
    blocking(move || jar_files::declared_id(&jar)).await
}

/// Runs `work` on the blocking pool, where jars are read and moved.
async fn blocking<T: Send + 'static>(work: impl FnOnce() -> T + Send + 'static) -> Result<T, String> {
    tokio::task::spawn_blocking(work).await.map_err(|error| error.to_string())
}

/// Whether the folder holds `file_name`, switched on or off.
fn held_by_name(mods: &Path, file_name: &str) -> bool {
    mods.join(file_name).is_file() || mods.join(format!("{file_name}.disabled")).is_file()
}

/// Whether a jar declaring the mod id `id` is already spoken for: the folder holds one, or this install planned one. A jar
/// that declares no id is never spoken for, so it is written as it always was.
fn spoken_for(id: Option<&str>, taken: &HashSet<String>) -> bool {
    id.is_some_and(|id| taken.contains(id))
}

/// Moves each staged jar to its target. A move that fails takes back the targets already moved, so the folder is as it was.
fn move_into(moves: &[(PathBuf, PathBuf)]) -> Result<(), String> {
    for (index, (from, to)) in moves.iter().enumerate() {
        if let Err(error) = move_file(from, to) {
            for (_, done) in &moves[..index] {
                let _ = std::fs::remove_file(done);
            }
            return Err(format!("moving {}: {error}", from.display()));
        }
    }
    Ok(())
}

/// How many staging folders are held now. Clear cache keeps the staging root while any is.
static HELD: AtomicUsize = AtomicUsize::new(0);

/// Whether an add holds a staging folder, so Clear cache must leave the staging root alone.
pub(crate) fn staging_in_use() -> bool {
    HELD.load(Ordering::Acquire) > 0
}

/// A fresh folder for the jars one add downloads before they are moved into `mods/`. It sits under the cache folder, or the
/// system temp folder when there is none, and it is removed when the add ends, whether or not the move worked.
struct Staging(PathBuf);

impl Staging {
    fn new() -> Result<Self, String> {
        let root = paths::cache_dir().unwrap_or_else(std::env::temp_dir).join("mod-staging");
        Self::within(&root)
    }

    /// A fresh folder inside `root`.
    fn within(root: &Path) -> Result<Self, String> {
        let path = root.join(format!("{:032x}", fastrand::u128(..)));
        std::fs::create_dir_all(&path).map_err(|error| error.to_string())?;
        HELD.fetch_add(1, Ordering::AcqRel);
        Ok(Self(path))
    }

    /// The folder for the jar at `index`, so two jars that share a file name cannot overwrite each other.
    fn dir(&self, index: usize) -> PathBuf {
        self.0.join(index.to_string())
    }
}

impl Drop for Staging {
    fn drop(&mut self) {
        let _ = std::fs::remove_dir_all(&self.0);
        HELD.fetch_sub(1, Ordering::AcqRel);
    }
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
