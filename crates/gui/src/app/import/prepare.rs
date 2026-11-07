//! Reads an import's source and makes the install that queues it. A modpack file or a CurseForge page goes through the
//! modpack install, and a Lodestone backup goes through the package install. Each answer comes with the record the dialog
//! lists. The reads open archives, so they run on the blocking pool.

use std::fs::File;
use std::path::{Path, PathBuf};

use chrono::Local;
use lodestone_export::format::lodestone::{self, Manifest};
use lodestone_platforms::{ContentType, Modpack, SearchQuery};

use super::launcher;
use super::page;
use super::record::{ImportFormat, ImportOrigin, ImportRecord};
use crate::app::discover::{MODRINTH, curseforge, prepare_modpack, resolve_curseforge_files};
use crate::app::install::package as install_package;
use crate::app::install::{InstallRequest, new_id, pack};
use crate::app::lang;
use crate::app::pages::discover::{Project, Source};
use crate::app::paths::LauncherPaths;

/// The largest source taken. A bigger file is refused before anything reads it.
pub const MAX_BYTES: u64 = 4 * 1024 * 1024 * 1024;

/// A file the dialog was given and has checked, as the dialog shows it.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Picked {
    /// Where the file is.
    pub path: PathBuf,
    /// The modpack's name, or the package's.
    pub name: String,
    /// The format the file was read as.
    pub format: ImportFormat,
    /// The file's size in bytes.
    pub bytes: u64,
}

/// An import ready to queue: the install that makes its instance, and the record the dialog lists.
#[derive(Debug, Clone)]
pub struct Prepared {
    pub request: InstallRequest,
    pub record: ImportRecord,
}

/// Checks a file the dialog was given, for the tab it was given on, and reads its name. A backup takes a `.lodestone`
/// package. A file takes a `.zip`, `.mrpack` or `.mcpack` modpack, which is read as a zip and told apart by its manifest.
pub async fn inspect(path: PathBuf, backup: bool) -> Result<Picked, String> {
    blocking(move || {
        let extension = path
            .extension()
            .and_then(|extension| extension.to_str())
            .map(str::to_ascii_lowercase)
            .unwrap_or_default();
        if backup {
            if extension != "lodestone" {
                return Err(lang::t("import.error.backup_type").to_string());
            }
            let bytes = size_of(&path)?;
            let manifest = read_package(&path)?;
            return Ok(Picked {
                path,
                name: manifest.name,
                format: ImportFormat::Lodestone,
                bytes,
            });
        }
        match extension.as_str() {
            "zip" | "mrpack" | "mcpack" => {}
            "lodestone" => return Err(lang::t("import.error.use_backup").to_string()),
            _ => return Err(lang::t("import.error.file_type").to_string()),
        }
        let bytes = size_of(&path)?;
        let (modpack, format) = read_modpack(&path)?;
        Ok(Picked {
            path,
            name: modpack.name,
            format,
            bytes,
        })
    })
    .await
}

/// Reads the modpack at `path` and makes the install that imports it.
pub async fn from_file(path: PathBuf, paths: LauncherPaths, taken: Vec<String>) -> Result<Prepared, String> {
    let read = path.clone();
    let (bytes, modpack, format) = blocking(move || {
        let bytes = size_of(&read)?;
        let (modpack, format) = read_modpack(&read)?;
        Ok((bytes, modpack, format))
    })
    .await?;
    let source = match format {
        ImportFormat::CurseForge => Source::CurseForge,
        _ => Source::Modrinth,
    };
    let name = modpack.name.clone();
    let request = pack::request(&paths, &taken, modpack, Some(path.clone()), Some(source), None, None).await?;
    Ok(Prepared {
        request,
        record: ImportRecord {
            name,
            format,
            origin: ImportOrigin::File,
            source: path_string(&path),
            bytes,
            imported_at: Local::now(),
        },
    })
}

/// Reads the Lodestone package at `path` and makes the install that restores it as a new instance. The package's bundled
/// files are laid over the instance by the install, and its linked files are downloaded.
pub async fn from_backup(path: PathBuf, paths: LauncherPaths, taken: Vec<String>) -> Result<Prepared, String> {
    let read = path.clone();
    let (bytes, manifest) = blocking(move || {
        let bytes = size_of(&read)?;
        let manifest = read_package(&read)?;
        Ok((bytes, manifest))
    })
    .await?;
    let mut payload = install_package::payload(&manifest, Some(path.as_path()));
    if payload.files.iter().any(|file| file.url.is_empty()) {
        payload.files = resolve_curseforge_files(payload.files).await?;
    }
    let request = install_package::request(&paths, &manifest, payload, new_id(), &taken);
    Ok(Prepared {
        request,
        record: ImportRecord {
            name: manifest.name,
            format: ImportFormat::Lodestone,
            origin: ImportOrigin::File,
            source: path_string(&path),
            bytes,
            imported_at: Local::now(),
        },
    })
}

/// Fetches the CurseForge modpack page `text` names, and makes the install of its newest file, or of the file the page
/// names. The archive is downloaded into the shared folder, as Discover downloads it.
pub async fn from_url(text: String, paths: LauncherPaths, taken: Vec<String>) -> Result<Prepared, String> {
    let page = page::parse(&text).ok_or_else(|| lang::t("import.url.invalid").to_string())?;
    let platform = curseforge()?;
    let mut query = SearchQuery::new(ContentType::Modpack);
    query.text = page.slug.clone();
    let results = platform.search(&query).await.map_err(|error| error.to_string())?;
    let hit = results
        .hits
        .into_iter()
        .find(|hit| hit.slug.as_deref() == Some(page.slug.as_str()))
        .ok_or_else(|| lang::t("import.url.not_found").to_string())?;
    let project = Project {
        source: Source::CurseForge,
        content: ContentType::Modpack,
        project_id: hit.id,
        name: hit.name,
        web_url: Some(page.project_url()),
        icon_url: hit.icon_url,
        game_versions: Vec::new(),
        version_id: page.file_id.clone(),
    };
    let (request, bytes) = prepare_modpack(project, paths, taken).await?;
    Ok(Prepared {
        record: ImportRecord {
            name: request.name.clone(),
            format: ImportFormat::CurseForge,
            origin: ImportOrigin::Url,
            source: page.address(),
            bytes,
            imported_at: Local::now(),
        },
        request,
    })
}

/// The import a dialog asks to read: a file of the modpack tab, a backup, an address, or a recorded import again.
#[derive(Debug, Clone)]
pub enum Work {
    File(PathBuf),
    Backup(PathBuf),
    Url(String),
    Again(ImportRecord),
    /// Instances the user checked in another launcher, each read from its folder.
    Launchers(Vec<launcher::Pick>),
}

/// Reads the import `work` describes and makes the install that queues it.
pub async fn read(work: Work, paths: LauncherPaths, taken: Vec<String>) -> Result<Vec<Prepared>, String> {
    match work {
        Work::File(path) => from_file(path, paths, taken).await.map(|one| vec![one]),
        Work::Backup(path) => from_backup(path, paths, taken).await.map(|one| vec![one]),
        Work::Url(address) => from_url(address, paths, taken).await.map(|one| vec![one]),
        Work::Again(record) => again(record, paths, taken).await,
        Work::Launchers(picks) => blocking(move || launcher::prepare(&picks, &paths, &taken)).await,
    }
}

/// Makes the install of a recorded import again, from the source it was started from.
async fn again(record: ImportRecord, paths: LauncherPaths, taken: Vec<String>) -> Result<Vec<Prepared>, String> {
    match (record.origin, record.format) {
        (ImportOrigin::Launcher, format) => {
            let launcher = launcher::Launcher::from_format(format).ok_or_else(|| lang::t("import.launcher.unrecognised").to_string())?;
            let pick = launcher::Pick {
                launcher,
                source: record.source,
            };
            blocking(move || launcher::prepare(&[pick], &paths, &taken)).await
        }
        (ImportOrigin::Url, _) => from_url(record.source, paths, taken).await.map(|one| vec![one]),
        (ImportOrigin::File, ImportFormat::Lodestone) => from_backup(PathBuf::from(record.source), paths, taken).await.map(|one| vec![one]),
        (ImportOrigin::File, _) => from_file(PathBuf::from(record.source), paths, taken).await.map(|one| vec![one]),
    }
}

/// Runs `work` on the blocking pool, since the reads it makes wait on the disk.
async fn blocking<T: Send + 'static>(work: impl FnOnce() -> Result<T, String> + Send + 'static) -> Result<T, String> {
    tokio::task::spawn_blocking(work).await.map_err(|error| error.to_string())?
}

/// The file's size in bytes, or the message that says why it is refused: it cannot be read, or it is too large.
fn size_of(path: &Path) -> Result<u64, String> {
    let bytes = std::fs::metadata(path)
        .map_err(|error| lang::t_with("import.error.unreadable", &[("error", error.to_string().as_str())]).to_string())?
        .len();
    check_size(bytes)
}

/// `bytes` when a source that size is taken, or the message that refuses it: it is over `MAX_BYTES`.
fn check_size(bytes: u64) -> Result<u64, String> {
    if bytes > MAX_BYTES {
        return Err(lang::t("import.error.too_large").to_string());
    }
    Ok(bytes)
}

/// Reads the modpack the zip at `path` holds, and the format its manifest names.
fn read_modpack(path: &Path) -> Result<(Modpack, ImportFormat), String> {
    let format = modpack_format(path)?;
    let modpack = MODRINTH.parse_modpack(path).map_err(|error| error.to_string())?;
    Ok((modpack, format))
}

/// The format of the manifest in the zip at `path`: `modrinth.index.json` for a `.mrpack`, `manifest.json` for a CurseForge
/// pack. A file that is no zip, or has neither, is not a modpack.
fn modpack_format(path: &Path) -> Result<ImportFormat, String> {
    let file = File::open(path).map_err(|error| lang::t_with("import.error.unreadable", &[("error", error.to_string().as_str())]).to_string())?;
    let mut zip = zip::ZipArchive::new(file).map_err(|_| lang::t("import.error.not_a_modpack").to_string())?;
    if zip.by_name("modrinth.index.json").is_ok() {
        Ok(ImportFormat::Mrpack)
    } else if zip.by_name("manifest.json").is_ok() {
        Ok(ImportFormat::CurseForge)
    } else {
        Err(lang::t("import.error.not_a_modpack").to_string())
    }
}

/// Reads the manifest of the Lodestone package at `path`.
fn read_package(path: &Path) -> Result<Manifest, String> {
    let file = File::open(path).map_err(|error| lang::t_with("import.error.unreadable", &[("error", error.to_string().as_str())]).to_string())?;
    lodestone::read_manifest(file)
        .map_err(|error| lang::t_with("import.error.backup_unreadable", &[("error", error.to_string().as_str())]).to_string())
}

/// The path as the record keeps it.
fn path_string(path: &Path) -> String {
    path.to_string_lossy().into_owned()
}

#[cfg(test)]
#[path = "../../../tests/app/import/prepare.rs"]
mod tests;
