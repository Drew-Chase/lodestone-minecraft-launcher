//! Adding a mod to an instance: the release that fits the instance, the required dependencies its `mods/` folder
//! lacks, and the jars written into that folder. Every release is chosen, and every dependency is downloaded and read,
//! before any jar lands in `mods/`, so a dependency with no release that fits, or a download that fails, leaves the
//! folder as it was. A dependency is recognised by the mod id its jar declares, so a copy the folder holds from another
//! platform is not written again. When that copy is another version, the user is asked which one the instance keeps: a
//! dependency version conflict. The answer names every jar the mod needs, its own first, as the platform source each
//! came from.

use std::collections::{HashSet, VecDeque};
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicUsize, Ordering};
use std::sync::{Arc, Mutex, PoisonError};

use lodestone_fs::move_file;
use lodestone_platforms::{ContentFile, ContentVersion, DependencyKind, VersionDependency};
use sipper::Sender as ProgressSender;
use tokio::sync::oneshot;
use tracing::warn;

use crate::app::database::Database;
use crate::app::discover::{download_version, versions_of};
use crate::app::install::ensure;
use crate::app::lang;
use crate::app::pages::discover::{AddRequest, Source};
use crate::app::pages::instance::files;
use crate::app::pages::instance::files::mods::{self as jar_files, Declared};
use crate::app::paths;
use crate::app::shared_library::ModSource;

/// A dependency the instance already has at another version than the one the mod being added brings. The add waits until
/// the user picks one: [`VersionConflict::answer`] with `true` uses the offered version and replaces the installed copy, and
/// `false`, or a question that goes unanswered, keeps the installed copy.
#[derive(Debug, Clone)]
pub struct VersionConflict {
    /// The dependency's name, as its jar declares it.
    pub dependency: String,
    /// The name of the mod being added.
    pub adding: String,
    /// The version the instance has.
    pub installed: String,
    /// The version the mod being added brings.
    pub offered: String,
    reply: Arc<Mutex<Option<oneshot::Sender<bool>>>>,
}

impl VersionConflict {
    /// Answers the question. Only the first answer counts.
    pub fn answer(&self, use_offered: bool) {
        let reply = self.reply.lock().unwrap_or_else(PoisonError::into_inner).take();
        if let Some(reply) = reply {
            let _ = reply.send(use_offered);
        }
    }
}

/// Adds the mod `request` names to the instance it names, writing its jar and the required dependencies the folder `mods`
/// does not already hold. A dependency is held when the folder has its file name, or a jar with its mod id, switched on or
/// off and from any platform. A held copy at another version is a conflict, which goes out through `ask`; when the user
/// takes the offered version, the installed copies are moved to the trash and their records in `db` are forgotten. The
/// answer names every jar the mod needs, the mod's own first. A dependency held by its file name is named as it always
/// was; one kept by its mod id is left out, so the copy already there keeps its record.
pub(crate) async fn add_to_instance(
    request: AddRequest,
    mods: PathBuf,
    db: Option<Database>,
    ask: &mut ProgressSender<VersionConflict>,
) -> Result<Vec<ModSource>, String> {
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
    let held = blocking({
        let mods = mods.clone();
        move || jar_files::held(&mods)
    })
    .await?;
    // The ids this install writes. A second jar with one of them is never written.
    let mut planned: HashSet<String> = HashSet::new();
    // The installed copies the user chose to replace with the version a dependency brings.
    let mut replaced: Vec<String> = Vec::new();

    // The mod's own jar comes first, and it is written whatever the folder holds, as it always was. Its id is planned, so
    // no dependency is written beside a second copy of the mod.
    let own_staged = stage(project.source, &chosen, &own, staging.dir(0)).await?;
    if let Some(found) = declared(own_staged.clone()).await? {
        planned.insert(found.id);
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
        let found = declared(staged.clone()).await?;
        let keep_installed = match &found {
            // A jar that declares no id cannot be matched, so it is written as it always was.
            None => false,
            Some(found) if planned.contains(&found.id) => true,
            Some(found) => {
                let copies: Vec<&(String, Declared)> = held.iter().filter(|(_, jar)| jar.id == found.id).collect();
                let versions: Vec<Option<&str>> = copies.iter().map(|(_, jar)| jar.version.as_deref()).collect();
                if copies.is_empty() {
                    false
                } else if let Some(installed) = conflicting_version(&versions, found.version.as_deref()) {
                    let dependency = found
                        .name
                        .clone()
                        .or_else(|| copies.iter().find_map(|(_, jar)| jar.name.clone()))
                        .unwrap_or_else(|| found.id.clone());
                    let offered = found.version.clone().unwrap_or_default();
                    let use_offered = ask_conflict(ask, dependency, project.name.clone(), installed, offered).await;
                    if use_offered {
                        replaced.extend(copies.iter().map(|(file_name, _)| file_name.clone()));
                    }
                    !use_offered
                } else {
                    // The same version, or one whose version cannot be compared: the installed copy stays, unasked.
                    true
                }
            }
        };
        let plan = if keep_installed {
            Plan::Held
        } else {
            if let Some(found) = found {
                planned.insert(found.id);
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
    let written: HashSet<PathBuf> = moves.iter().map(|(_, target)| target.clone()).collect();
    blocking(move || move_into(&moves)).await??;

    // The copies the user replaced go to the trash only once the versions that replace them are in place.
    replaced.retain(|file_name| !written.contains(&mods.join(file_name)));
    if !replaced.is_empty() {
        let folder = mods.clone();
        let names = replaced.clone();
        let failures = blocking(move || {
            names
                .iter()
                .filter_map(|name| files::move_to_trash(&folder.join(name)).err().map(|error| format!("{name}: {error}")))
                .collect::<Vec<_>>()
        })
        .await?;
        for failure in failures {
            warn!("moving a replaced dependency to the trash failed: {failure}");
        }
        if let Some(db) = &db {
            for file_name in &replaced {
                if let Err(error) = db.forget_mod_source(&instance.id, file_name).await {
                    warn!("forgetting the record of a replaced dependency failed: {error}");
                }
            }
        }
    }

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
                version_number: Some(jar.release.version_number),
                name,
                file_name: jar.file.name,
                sha1: jar.file.sha1,
            })
        })
        .collect())
}

/// Sends a dependency version conflict out through `ask` and waits for the answer: `true` to use the offered version.
async fn ask_conflict(ask: &mut ProgressSender<VersionConflict>, dependency: String, adding: String, installed: String, offered: String) -> bool {
    let (reply, answer) = oneshot::channel();
    ask.send(VersionConflict {
        dependency,
        adding,
        installed,
        offered,
        reply: Arc::new(Mutex::new(Some(reply))),
    })
    .await;
    answer.await.unwrap_or(false)
}

/// The installed version that conflicts with `offered`, or `None` when there is nothing to ask: a copy already has the
/// offered version, or a version on either side is not declared, so the two cannot be compared.
fn conflicting_version(installed: &[Option<&str>], offered: Option<&str>) -> Option<String> {
    let offered = offered?;
    let known: Vec<&str> = installed.iter().copied().flatten().collect();
    if known.contains(&offered) {
        return None;
    }
    known.first().map(|version| (*version).to_string())
}

/// What one install does with a jar the mod needs.
enum Plan {
    /// The jar is written into `mods/` from its staged download.
    Write(PathBuf),
    /// The folder already holds the file under its name, so nothing is written. The record is written as it always was.
    Named,
    /// A jar with the same mod id is kept, from the folder or planned by this install, so nothing is written or recorded.
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

/// What a staged jar declares, read on the blocking pool.
async fn declared(jar: PathBuf) -> Result<Option<Declared>, String> {
    blocking(move || jar_files::declared(&jar)).await
}

/// Runs `work` on the blocking pool, where jars are read and moved.
async fn blocking<T: Send + 'static>(work: impl FnOnce() -> T + Send + 'static) -> Result<T, String> {
    tokio::task::spawn_blocking(work).await.map_err(|error| error.to_string())
}

/// Whether the folder holds `file_name`, switched on or off.
fn held_by_name(mods: &Path, file_name: &str) -> bool {
    mods.join(file_name).is_file() || mods.join(format!("{file_name}.disabled")).is_file()
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
