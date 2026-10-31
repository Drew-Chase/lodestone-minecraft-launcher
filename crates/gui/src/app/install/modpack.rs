//! The modpack stage of an install: the mod files a modpack lists, fetched into
//! the instance, and the archives it lays over them. The download checks each
//! file against the hash the pack gives it.

use std::io::Write;
use std::path::{Component, Path, PathBuf};

use lodestone_platforms::{Modpack, ModpackFile};
use sipper::Sender as ProgressSender;
use tokio::sync::mpsc;
use tracing::warn;

use super::download::{self, Part, PartReport};
use super::{InstallProgress, Stage, Totals, drive, fraction, report};
use crate::app::lang;
use crate::app::launch::libraries::Item;

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
) -> Result<(), String> {
    report(progress, Stage::Modpack, None, lang::t("install.modpack.files").to_string()).await;
    let fetched = game_dir.join(FETCHED);
    // Where each unpack's archive is: its file on disk, or the place a remote one is fetched to.
    let archives: Vec<PathBuf> = payload
        .unpacks
        .iter()
        .enumerate()
        .map(|(index, unpack)| match &unpack.origin {
            Origin::Local(path) => path.clone(),
            Origin::Remote { .. } => fetched.join(format!("{index}.zip")),
        })
        .collect();
    let mut items: Vec<Item> = payload
        .files
        .iter()
        .map(|file| Item {
            url: Some(file.url.clone()),
            path: game_dir.join(&file.path),
            sha1: file.sha1.clone(),
            size: file.size,
        })
        .collect();
    items.extend(payload.unpacks.iter().zip(&archives).filter_map(|(unpack, path)| match &unpack.origin {
        Origin::Remote { url, sha1, size } => Some(Item {
            url: Some(url.clone()),
            path: path.clone(),
            sha1: sha1.clone(),
            size: *size,
        }),
        Origin::Local(_) => None,
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
    tokio::task::spawn_blocking(move || unpack_all(&unpacks, &game_dir))
        .await
        .map_err(|error| error.to_string())??;
    Ok(())
}

/// Unpacks each archive into the instance in turn, then removes the folder the remote archives were
/// fetched into.
fn unpack_all(unpacks: &[(PathBuf, Unpack)], game_dir: &Path) -> Result<(), String> {
    for (archive, unpack) in unpacks {
        extract(archive, &unpack.folder, &unpack.target, &unpack.skip, game_dir)?;
    }
    let fetched = game_dir.join(FETCHED);
    if let Err(error) = std::fs::remove_dir_all(&fetched)
        && error.kind() != std::io::ErrorKind::NotFound
    {
        warn!("could not remove {}: {error}", fetched.display());
    }
    Ok(())
}

/// Extracts the entries under `folder` in `archive` into `game_dir/target`, with the folder removed from
/// their paths. Entries that `skip` names are left out, and an entry whose path would leave the target
/// is skipped.
fn extract(archive: &Path, folder: &str, target: &str, skip: &[String], game_dir: &Path) -> Result<(), String> {
    let file = std::fs::File::open(archive).map_err(|error| error.to_string())?;
    let mut zip = zip::ZipArchive::new(file).map_err(|error| error.to_string())?;
    let prefix = folder_prefix(folder);
    let destination = game_dir.join(target);
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
        std::io::copy(&mut entry, &mut out).map_err(|error| error.to_string())?;
        out.flush().map_err(|error| error.to_string())?;
    }
    Ok(())
}

/// The prefix the entries of `folder` carry in an archive: none for the archive's root, and `folder/`
/// otherwise.
fn folder_prefix(folder: &str) -> String {
    match folder.trim_matches('/') {
        "" => String::new(),
        trimmed => format!("{trimmed}/"),
    }
}

#[cfg(test)]
#[path = "../../../tests/app/install/modpack.rs"]
mod tests;
