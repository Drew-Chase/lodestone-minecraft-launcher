//! The modpack stage of an install: the mod files a modpack lists, fetched into
//! the instance, and the archives it lays over them. The download checks each
//! file against the hash the pack gives it.

use std::io::Write;
use std::path::{Component, Path, PathBuf};
use std::sync::Arc;
use std::sync::atomic::{AtomicBool, Ordering};

use lodestone_export::format::lodestone;
use lodestone_platforms::{Modpack, ModpackFile};
use sipper::Sender as ProgressSender;
use tokio::sync::mpsc;
use tracing::warn;

use super::copy;
use super::download::{self, Part, PartReport};
use super::{InstallProgress, Stage, Totals, drive, fraction, report};
use crate::app::lang;
use crate::app::launch::libraries::Item;
use crate::app::shared_library::{AddedContent, ModSource};

/// The folder under the instance that a remote archive is fetched into until it is unpacked.
const FETCHED: &str = ".modpack";

/// What a modpack install adds to the instance beyond its loader.
#[derive(Debug, Clone)]
pub struct Payload {
    /// The files the pack places, each with the path it goes to in the instance and the url it
    /// comes from.
    pub files: Vec<ModpackFile>,
    /// The archives the pack lays into the instance, in the order they unpack.
    pub unpacks: Vec<Unpack>,
    /// The platform project and release of each mod the pack places, recorded against the instance once it is
    /// installed. The records name no instance until then.
    pub sources: Vec<ModSource>,
    /// The platform project and release of each pack the pack places in its resource or shader folder, recorded as the
    /// instance's content once it is installed. The records name no instance until then.
    pub packs: Vec<AddedContent>,
}

/// An archive the install lays into the instance. The entries under `folder` go to `target`, except
/// the ones `skip` names.
#[derive(Debug, Clone)]
pub struct Unpack {
    /// Where the archive is: a file on disk, or a url the install fetches first.
    pub origin: Origin,
    /// The folder inside the archive whose entries are taken. Empty takes the whole archive.
    pub folder: String,
    /// The folder of the instance the entries go into. Empty is the instance's root.
    pub target: String,
    /// Entries, by their path inside the archive, that are left out.
    pub skip: Vec<String>,
}

/// Where an archive a modpack unpacks comes from.
#[derive(Debug, Clone)]
pub enum Origin {
    /// A file already on disk, such as the pack the install was started from.
    Local(PathBuf),
    /// A file the install fetches into the instance, then unpacks and removes. The hash and size the
    /// pack gives it, when it gives them, are checked.
    Remote { url: String, sha1: Option<String>, size: Option<u64> },
    /// A Lodestone package on disk. Its bundled files, under `overrides/`, lay over the root of the instance. The
    /// folder, target and skip of the unpack do not apply to a package.
    Package(PathBuf),
    /// A folder on disk, such as a launcher's game folder. Its files are copied into the instance, except the top-level
    /// names in `skip`, which are left out along with the launcher's own files.
    Folder { root: PathBuf, skip: Vec<String> },
}

impl Payload {
    /// The payload of a modpack. `pack_file` is the pack's own file on disk, which its overrides come
    /// from, when it has one. The overrides go last, so they lay over the other archives as they lay
    /// over the mods.
    pub fn from_modpack(modpack: &Modpack, pack_file: Option<PathBuf>) -> Payload {
        let mut unpacks: Vec<Unpack> = modpack
            .archives
            .iter()
            .map(|archive| Unpack {
                origin: Origin::Remote {
                    url: archive.url.clone(),
                    sha1: archive.sha1.clone(),
                    size: archive.size,
                },
                folder: archive.prefix.clone(),
                target: archive.target.clone(),
                skip: Vec::new(),
            })
            .collect();
        if let (Some(folder), Some(path)) = (&modpack.overrides, pack_file) {
            unpacks.push(Unpack {
                origin: Origin::Local(path),
                folder: folder.clone(),
                target: String::new(),
                skip: modpack.overrides_skip.clone(),
            });
        }
        Payload {
            files: modpack.files.clone(),
            unpacks,
            sources: Vec::new(),
            packs: Vec::new(),
        }
    }
}

/// The modpack stage: fetches the pack's files, and its remote archives, into `game_dir`, then unpacks
/// the archives over them. A file that is already on disk at the size the pack expects is not fetched
/// again.
pub(super) async fn stage(
    progress: &mut ProgressSender<InstallProgress>,
    payload: &Payload,
    game_dir: &Path,
    client: &reqwest::Client,
) -> Result<Vec<String>, String> {
    report(progress, Stage::Modpack, None, lang::t("install.modpack.files").to_string()).await;
    let fetched = game_dir.join(FETCHED);
    // Where each unpack's archive is: its file on disk, or the place a remote one is fetched to.
    let archives: Vec<PathBuf> = payload
        .unpacks
        .iter()
        .enumerate()
        .map(|(index, unpack)| match &unpack.origin {
            Origin::Local(path) | Origin::Package(path) => path.clone(),
            Origin::Folder { root, .. } => root.clone(),
            Origin::Remote { .. } => fetched.join(format!("{index}.zip")),
        })
        .collect();
    // A pack's paths are written by whoever made it, so each one must stay inside the instance before anything is fetched.
    let mut items: Vec<Item> = Vec::with_capacity(payload.files.len());
    for file in &payload.files {
        let path = lodestone_platforms::paths::contained(game_dir, &file.path)
            .ok_or_else(|| lang::t_with("install.unsafe_path", &[("path", file.path.as_str())]))?;
        items.push(Item {
            url: Some(file.url.clone()),
            path,
            sha1: file.sha1.clone(),
            size: file.size,
        });
    }
    items.extend(payload.unpacks.iter().zip(&archives).filter_map(|(unpack, path)| match &unpack.origin {
        Origin::Remote { url, sha1, size } => Some(Item {
            url: Some(url.clone()),
            path: path.clone(),
            sha1: sha1.clone(),
            size: *size,
        }),
        Origin::Local(_) | Origin::Package(_) | Origin::Folder { .. } => None,
    }));
    // The instance's folders are written at the end of the install, so the folders the items go into
    // are made here.
    let parents: Vec<PathBuf> = items.iter().filter_map(|item| item.path.parent().map(Path::to_path_buf)).collect();
    tokio::task::spawn_blocking(move || {
        for parent in parents {
            if let Err(error) = std::fs::create_dir_all(&parent) {
                warn!("could not create {}: {error}", parent.display());
            }
        }
    })
    .await
    .map_err(|error| error.to_string())?;
    let missing = download::missing(items).await?;
    let total = missing.present + missing.missing.len();
    let (reports_tx, reports) = mpsc::channel::<PartReport>(256);
    let mut totals = Totals::default();
    let work = download::fetch(client, missing.missing, Part::Mods, missing.present, total, &reports_tx);
    drive(
        progress,
        reports,
        move |report| {
            let sum = totals.apply(report);
            InstallProgress {
                stage: Stage::Modpack,
                fraction: fraction(sum.done, sum.total),
                detail: lang::t_with(
                    "install.files_count",
                    &[("done", sum.done.to_string().as_str()), ("total", sum.total.to_string().as_str())],
                ),
                transfer: sum.transfer,
            }
        },
        work,
    )
    .await?;

    let unpacks: Vec<(PathBuf, Unpack)> = archives.into_iter().zip(payload.unpacks.iter().cloned()).collect();
    let game_dir = game_dir.to_path_buf();
    // Dropping this stage (an aborted install) sets the flag, so a copy still running stops at its next file.
    let abandon = Abandon(Arc::new(AtomicBool::new(false)));
    let abandoned = Arc::clone(&abandon.0);
    let shipped = tokio::task::spawn_blocking(move || unpack_all(&unpacks, &game_dir, &abandoned))
        .await
        .map_err(|error| error.to_string())??;
    Ok(shipped)
}

/// Unpacks each archive into the instance in turn, then removes the folder the remote archives were
/// fetched into. Returns the paths, relative to the instance, of the files the archives wrote.
fn unpack_all(unpacks: &[(PathBuf, Unpack)], game_dir: &Path, abandoned: &AtomicBool) -> Result<Vec<String>, String> {
    let mut shipped = Vec::new();
    for (archive, unpack) in unpacks {
        match &unpack.origin {
            Origin::Package(_) => extract_package(archive, game_dir)?,
            Origin::Folder { root, skip } => copy_folder(root, skip, game_dir, abandoned)?,
            _ => shipped.extend(extract(archive, &unpack.folder, &unpack.target, &unpack.skip, game_dir)?),
        }
    }
    let fetched = game_dir.join(FETCHED);
    if let Err(error) = std::fs::remove_dir_all(&fetched)
        && error.kind() != std::io::ErrorKind::NotFound
    {
        warn!("could not remove {}: {error}", fetched.display());
    }
    Ok(shipped)
}

/// Extracts the entries under `folder` in `archive` into `game_dir/target`, with the folder removed from
/// their paths. Entries that `skip` names are left out, and an entry whose path would leave the target
/// is skipped.
fn extract(archive: &Path, folder: &str, target: &str, skip: &[String], game_dir: &Path) -> Result<Vec<String>, String> {
    let file = std::fs::File::open(archive).map_err(|error| error.to_string())?;
    let archive_len = file.metadata().map_err(|error| error.to_string())?.len();
    let mut zip = zip::ZipArchive::new(file).map_err(|error| error.to_string())?;
    let declared = (0..zip.len())
        .filter_map(|index| zip.by_index_raw(index).ok().map(|entry| entry.size()))
        .fold(0, u64::saturating_add);
    if !lodestone_fs::archive_fits(declared, archive_len) {
        return Err(format!("the archive declares {declared} bytes from {archive_len}"));
    }
    let prefix = folder_prefix(folder);
    let destination = game_dir.join(target);
    let mut written = Vec::new();
    for index in 0..zip.len() {
        let mut entry = zip.by_index(index).map_err(|error| error.to_string())?;
        if entry.is_dir() {
            continue;
        }
        let name = entry.name().map_err(|error| error.to_string())?.into_owned();
        if skip.contains(&name) {
            continue;
        }
        let Some(relative) = name.strip_prefix(&prefix).map(str::to_owned) else {
            continue;
        };
        let inside = Path::new(&relative).components().all(|part| matches!(part, Component::Normal(_)));
        if relative.is_empty() || !inside {
            continue;
        }
        let path = destination.join(&relative);
        if let Some(parent) = path.parent() {
            std::fs::create_dir_all(parent).map_err(|error| error.to_string())?;
        }
        let mut out = std::fs::File::create(&path).map_err(|error| error.to_string())?;
        let (declared, compressed) = (entry.size(), entry.compressed_size());
        lodestone_fs::copy_entry(&mut entry, &mut out, declared, compressed).map_err(|error| error.to_string())?;
        out.flush().map_err(|error| error.to_string())?;
        written.push(in_instance(target, &relative));
    }
    Ok(written)
}

/// The path of `relative`, which sits inside `target`, relative to the instance folder, with `/` separators.
fn in_instance(target: &str, relative: &str) -> String {
    match target.trim_matches('/') {
        "" => relative.to_string(),
        trimmed => format!("{trimmed}/{relative}"),
    }
}

/// Lays the bundled files of a Lodestone package over `game_dir`, from the `overrides/` folder inside it.
fn extract_package(archive: &Path, game_dir: &Path) -> Result<(), String> {
    let file = std::fs::File::open(archive).map_err(|error| error.to_string())?;
    lodestone::extract_overrides(file, game_dir).map_err(|error| error.to_string())
}

/// Copies a launcher's game folder into the instance, leaving out the names in `skip` and the launcher's own files.
fn copy_folder(root: &Path, skip: &[String], game_dir: &Path, abandoned: &AtomicBool) -> Result<(), String> {
    let planned = copy::plan(root, skip).map_err(|error| error.to_string())?;
    let report = copy::copy_into(&planned, game_dir, abandoned).map_err(|error| error.to_string())?;
    if report.failed > 0 {
        warn!("{} of {} files could not be copied from {}", report.failed, planned.len(), root.display());
    }
    Ok(())
}

/// Sets its flag when it is dropped. An install holds one, so an install that is dropped (aborted) tells a copy still
/// running on the blocking pool to stop at its next file.
struct Abandon(Arc<AtomicBool>);

impl Drop for Abandon {
    fn drop(&mut self) {
        self.0.store(true, Ordering::Relaxed);
    }
}

/// The prefix the entries of `folder` carry in an archive: none for the archive's root, and `folder/`
/// otherwise.
pub(super) fn folder_prefix(folder: &str) -> String {
    match folder.trim_matches('/') {
        "" => String::new(),
        trimmed => format!("{trimmed}/"),
    }
}

#[cfg(test)]
#[path = "../../../tests/app/install/modpack.rs"]
mod tests;
