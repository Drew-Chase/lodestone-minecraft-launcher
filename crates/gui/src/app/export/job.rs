//! The export job: the background work that turns an instance into a package. It reads the selected files,
//! hashes the mods and resource packs, finds the links the platforms know them by, and writes the package to a
//! partial file that is renamed into place once it is whole. A cancel stops the job at the next file, and the
//! partial file is removed.

use std::collections::HashMap;
use std::fs::File;
use std::io::BufWriter;
use std::path::{Path, PathBuf};
use std::sync::Arc;
use std::sync::atomic::{AtomicBool, Ordering};
use std::time::{SystemTime, UNIX_EPOCH};

use lodestone_export::format::{Format, Metadata, Package, curseforge, is_bundled, lodestone, mrpack, plain};
use lodestone_export::hash::{Digests, digest_file};
use lodestone_export::link::{CurseForgeLink, Entry, Links, ModrinthLink};
use lodestone_export::plan::{self, Planned, Selection};
use lodestone_export::progress::{Control, Progress};
use lodestone_export::{Error as ExportError, portable};
use lodestone_platforms::ContentVersion;
use sipper::{Sender as ProgressSender, Sipper, sipper};
use tokio::sync::mpsc;
use tracing::{info, warn};

use crate::app::database::Database;
use crate::app::discover::MODRINTH;
use crate::app::lang;
use crate::app::pages::discover::Source;
use crate::app::shared_library::ModSource;

/// The folders whose archives a platform can identify by their hash.
const LINKABLE_FOLDERS: [&str; 4] = ["mods", "resourcepacks", "shaderpacks", "datapacks"];

/// What an export writes, and where.
#[derive(Debug, Clone)]
pub struct Request {
    /// The instance's id, which its mod records are kept under.
    pub instance: String,
    /// The instance's folder.
    pub folder: PathBuf,
    /// The paths the user left on.
    pub selection: Selection,
    pub format: Format,
    pub metadata: Metadata,
    /// Whether a portable launcher bundles every mod, rather than leaving the linked ones to download.
    pub offline: bool,
    /// Where the package is written, once it is whole.
    pub target: PathBuf,
    /// The launcher's own file, which a portable launcher is copied from.
    pub executable: PathBuf,
}

/// The stage an export is in.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Stage {
    Reading,
    Hashing,
    Identifying,
    Writing,
}

impl Stage {
    /// The stage's name, as the popover shows it.
    pub fn label(self) -> &'static str {
        match self {
            Stage::Reading => lang::t("export.stage.reading"),
            Stage::Hashing => lang::t("export.stage.hashing"),
            Stage::Identifying => lang::t("export.stage.identifying"),
            Stage::Writing => lang::t("export.stage.writing"),
        }
    }
}

/// How far an export has got: its stage, and how many of the stage's files are done.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct ExportProgress {
    pub stage: Stage,
    pub done: usize,
    pub total: usize,
}

impl ExportProgress {
    /// The share of the stage that is done, when the stage knows its total.
    pub fn fraction(self) -> Option<f32> {
        (self.total > 0).then(|| self.done as f32 / self.total as f32)
    }
}

/// An export as the downloads queue holds it.
#[derive(Debug, Clone)]
pub struct ExportJob {
    /// The instance's name, which the popover shows.
    pub name: String,
    pub format: Format,
    /// Where the package is written.
    pub target: PathBuf,
    /// Set by a cancel. The job checks it between files.
    pub cancel: Arc<AtomicBool>,
    /// The latest report from the job.
    pub progress: Option<ExportProgress>,
}

/// A mod or resource archive with its digests.
struct Hashed {
    path: String,
    digests: Digests,
}

/// Writes the package for `request` to its target, reporting progress, and returns the target. A cancel, or a
/// failure, leaves no partial file behind and returns the reason.
pub fn run(request: Request, db: Database, cancel: Arc<AtomicBool>) -> impl Sipper<Result<PathBuf, String>, ExportProgress> {
    sipper(move |mut progress: ProgressSender<ExportProgress>| async move { export(request, db, cancel, &mut progress).await })
}

async fn export(request: Request, db: Database, cancel: Arc<AtomicBool>, progress: &mut ProgressSender<ExportProgress>) -> Result<PathBuf, String> {
    report(progress, Stage::Reading, 0, 0).await;
    let (folder, selection) = (request.folder.clone(), request.selection.clone());
    let planned = blocking(move || plan::expand(&folder, &selection))
        .await?
        .map_err(|error| error.to_string())?;
    check(&cancel)?;

    let hashed = hash_linkable(progress, &planned, &cancel).await?;
    check(&cancel)?;

    report(progress, Stage::Identifying, 0, 0).await;
    let links = identify(&db, &request.instance, &hashed).await;
    check(&cancel)?;

    let entries = entries(planned, hashed, links);
    write(progress, &request, entries, cancel).await
}

/// Hashes the linkable archives among `planned`, on the blocking pool. A file that cannot be read fails the
/// export, since a package with a wrong digest would not match the platforms.
async fn hash_linkable(progress: &mut ProgressSender<ExportProgress>, planned: &[Planned], cancel: &Arc<AtomicBool>) -> Result<Vec<Hashed>, String> {
    let candidates: Vec<Planned> = planned.iter().filter(|file| is_linkable(&file.path)).cloned().collect();
    let total = candidates.len();
    let cancel = cancel.clone();
    forward(progress, move |reports| {
        let mut hashed = Vec::with_capacity(total);
        for (index, file) in candidates.into_iter().enumerate() {
            if cancel.load(Ordering::SeqCst) {
                return Err(ExportError::Cancelled);
            }
            let digests = digest_file(&file.source)?;
            hashed.push(Hashed { path: file.path, digests });
            let _ = reports.send(ExportProgress {
                stage: Stage::Hashing,
                done: index + 1,
                total,
            });
        }
        Ok(hashed)
    })
    .await?
    .map_err(|error| error.to_string())
}

/// The links of each hashed file. Its Modrinth release is found by its SHA-1 in one batched call, and its
/// CurseForge file comes from the record the install kept, when that record still matches the bytes. A failed
/// lookup leaves the file unlinked, so the package still comes out and bundles the file.
async fn identify(db: &Database, instance: &str, hashed: &[Hashed]) -> HashMap<String, Links> {
    let records = match db.mod_sources_of(instance).await {
        Ok(records) => records,
        Err(error) => {
            warn!("reading the mod records for the export failed: {error}");
            Vec::new()
        }
    };
    let hashes: Vec<String> = hashed.iter().map(|file| file.digests.sha1.clone()).collect();
    let releases = if hashes.is_empty() {
        HashMap::new()
    } else {
        MODRINTH.versions_by_hashes(&hashes).await.unwrap_or_else(|error| {
            warn!("identifying the export's mods on Modrinth failed: {error}");
            HashMap::new()
        })
    };
    hashed
        .iter()
        .map(|file| {
            let links = Links {
                modrinth: releases
                    .get(&file.digests.sha1)
                    .and_then(|release| modrinth_link(release, &file.digests.sha1)),
                curseforge: curseforge_link(&records, &file.path, &file.digests.sha1),
            };
            (file.path.clone(), links)
        })
        .collect()
}

/// The Modrinth link of the file with `sha1`, from the release Modrinth matched it to. A release whose files do not
/// include that hash gives no link, since the link would download different bytes.
fn modrinth_link(release: &ContentVersion, sha1: &str) -> Option<ModrinthLink> {
    let project = release.project_id.as_ref()?;
    let file = release
        .files
        .iter()
        .find(|file| file.sha1.as_deref().is_some_and(|hash| hash.eq_ignore_ascii_case(sha1)))?;
    (!file.url.is_empty()).then(|| ModrinthLink {
        project: project.clone(),
        version: release.id.clone(),
        page: format!("https://modrinth.com/project/{project}"),
        download_url: file.url.clone(),
    })
}

/// The CurseForge link of the mod at `path`, when the install recorded a CurseForge file for it and that file's
/// hash is `sha1`.
fn curseforge_link(records: &[ModSource], path: &str, sha1: &str) -> Option<CurseForgeLink> {
    let file_name = path.strip_prefix("mods/")?;
    records
        .iter()
        .find(|record| {
            record.platform == Source::CurseForge.key()
                && record.file_name == file_name
                && record.sha1.as_deref().is_some_and(|recorded| recorded.eq_ignore_ascii_case(sha1))
        })
        .map(|record| CurseForgeLink {
            project: record.project_id.clone(),
            file: record.version_id.clone(),
        })
}

/// The entries a package takes: each planned file, with the digests and links found for it.
fn entries(planned: Vec<Planned>, hashed: Vec<Hashed>, mut links: HashMap<String, Links>) -> Vec<Entry> {
    let mut digests: HashMap<String, Digests> = hashed.into_iter().map(|file| (file.path, file.digests)).collect();
    planned
        .into_iter()
        .map(|file| Entry {
            digests: digests.remove(&file.path),
            links: links.remove(&file.path).unwrap_or_default(),
            path: file.path,
            source: file.source,
            size: file.size,
        })
        .collect()
}

/// Writes the package for `entries` to a partial file beside the target, then renames it into place.
async fn write(
    progress: &mut ProgressSender<ExportProgress>,
    request: &Request,
    entries: Vec<Entry>,
    cancel: Arc<AtomicBool>,
) -> Result<PathBuf, String> {
    let partial = partial_path(&request.target);
    let (format, metadata, executable) = (request.format, request.metadata.clone(), request.executable.clone());
    let bundle_linked = request.offline && format == Format::Portable;
    let counted = entries.iter().filter(|entry| is_bundled(format, entry, bundle_linked));
    let (total_files, total_bytes) = (counted.clone().count(), counted.map(|entry| entry.size).sum::<u64>());
    let job_partial = partial.clone();
    let written = forward(progress, move |reports| {
        let mut report = |state: Progress| {
            let _ = reports.send(ExportProgress {
                stage: Stage::Writing,
                done: state.files,
                total: state.total_files,
            });
        };
        let mut control = Control::new(&cancel, total_files, total_bytes, &mut report);
        let package = Package {
            id: new_id(),
            exported_at: now(),
            metadata: &metadata,
            entries: &entries,
            bundle_linked,
        };
        match format {
            Format::Mrpack => mrpack::write(&job_partial, &metadata, &entries, &mut control),
            Format::CurseForge => curseforge::write(&job_partial, &metadata, &entries, &mut control),
            Format::Zip => plain::write(&job_partial, &package, &mut control),
            Format::Lodestone => File::create(&job_partial)
                .map_err(ExportError::from)
                .and_then(|file| lodestone::write_to(BufWriter::new(file), &package, &mut control).map(drop)),
            Format::Portable => write_portable(&executable, &job_partial, &metadata.name, &package, &mut control),
        }
    })
    .await?;
    if let Err(error) = written {
        remove_partial(&partial);
        return Err(error.to_string());
    }
    if let Err(error) = std::fs::rename(&partial, &request.target) {
        remove_partial(&partial);
        return Err(error.to_string());
    }
    info!("Exported {}", request.target.display());
    Ok(request.target.clone())
}

/// Writes a portable launcher: an app bundle on macOS, and a copy of the launcher with the package after it
/// elsewhere.
fn write_portable(executable: &Path, partial: &Path, name: &str, package: &Package<'_>, control: &mut Control<'_>) -> Result<(), ExportError> {
    if cfg!(target_os = "macos") {
        let bundle_id = format!("dev.lodestone.portable.{}", package.id);
        portable::write_app(executable, partial, &file_safe(name), &bundle_id, package, control)
    } else {
        portable::write_binary(executable, partial, package, control)
    }
}

/// Whether `path` is a mod or resource archive in one of the folders a platform can identify.
fn is_linkable(path: &str) -> bool {
    let Some((folder, name)) = path.split_once('/') else {
        return false;
    };
    LINKABLE_FOLDERS.contains(&folder) && !name.contains('/') && (name.ends_with(".jar") || name.ends_with(".zip"))
}

/// The name with the characters a file name cannot hold replaced, so it can name a file or a bundle.
pub fn file_safe(name: &str) -> String {
    let safe: String = name
        .chars()
        .map(|character| {
            if character.is_control() || matches!(character, '/' | '\\' | ':' | '*' | '?' | '"' | '<' | '>' | '|') {
                '_'
            } else {
                character
            }
        })
        .collect();
    let trimmed = safe.trim();
    if trimmed.is_empty() { "export".to_string() } else { trimmed.to_string() }
}

/// The file a package is written to before it is renamed: the target with `.partial` appended.
fn partial_path(target: &Path) -> PathBuf {
    let mut partial = target.as_os_str().to_owned();
    partial.push(".partial");
    PathBuf::from(partial)
}

fn remove_partial(partial: &Path) {
    if let Err(error) = std::fs::remove_file(partial)
        && error.kind() != std::io::ErrorKind::NotFound
    {
        warn!("could not remove the partial export {}: {error}", partial.display());
    }
}

/// A random id for the package, which a portable launcher keeps its folders under.
fn new_id() -> String {
    format!("{:032x}", fastrand::u128(..))
}

fn now() -> u64 {
    SystemTime::now().duration_since(UNIX_EPOCH).map_or(0, |since| since.as_secs())
}

/// Fails once the job has been cancelled.
fn check(cancel: &AtomicBool) -> Result<(), String> {
    if cancel.load(Ordering::SeqCst) {
        return Err("cancelled".to_string());
    }
    Ok(())
}

async fn report(progress: &mut ProgressSender<ExportProgress>, stage: Stage, done: usize, total: usize) {
    progress.send(ExportProgress { stage, done, total }).await;
}

/// Runs `work` on the blocking pool, and forwards the reports it sends until it returns.
async fn forward<T: Send + 'static>(
    progress: &mut ProgressSender<ExportProgress>,
    work: impl FnOnce(mpsc::UnboundedSender<ExportProgress>) -> T + Send + 'static,
) -> Result<T, String> {
    let (reports, mut received) = mpsc::unbounded_channel();
    let handle = tokio::task::spawn_blocking(move || work(reports));
    while let Some(report) = received.recv().await {
        progress.send(report).await;
    }
    handle.await.map_err(|error| error.to_string())
}

/// Runs `work` on the blocking pool.
async fn blocking<T: Send + 'static>(work: impl FnOnce() -> T + Send + 'static) -> Result<T, String> {
    tokio::task::spawn_blocking(work).await.map_err(|error| error.to_string())
}

#[cfg(test)]
#[path = "../../../tests/app/export/job.rs"]
mod tests;
