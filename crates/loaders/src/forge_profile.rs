//! Forge's install profile, and the install a `versionInfo` profile needs without running its installer.
//!
//! Forge 1.5.2 to 1.12.1 ship an installer with no `--installClient`: the installer opens its window and installs the client
//! from there. This module does that work directly. It extracts the universal jar the profile names to the maven path the
//! profile gives it, downloads the libraries the window downloads (those marked `clientreq`), and writes the profile's
//! `versionInfo` as the version JSON. Forge 1.12.2 and later install through their processors, and the installer runs them.

use std::io::Read;
use std::path::{Path, PathBuf};

use futures_util::{StreamExt, TryStreamExt};
use lodestone_net::download::{Target, sha1_of_file};
use serde_json::Value;
use simple_download_utility::DownloadProgress;
use tokio::sync::mpsc::Sender;
use zipcrawl::ZipManager;

use crate::error::LoaderError;
use crate::http::{download_files, maven_base, maven_jar_path};
use crate::install::{InstalledVersion, write_version};

/// The loader name the errors of this module carry.
const LOADER: &str = "forge";

/// The entry of an installer jar that holds its install profile.
const PROFILE: &str = "install_profile.json";

/// The base a library downloads from when its profile entry names no `url`. The window treats a failure here as non-fatal.
const DEFAULT_LIBRARY_URL: &str = "https://libraries.minecraft.net/";

/// How many libraries download at once.
const CONCURRENCY: usize = 8;

/// The layout of a Forge installer's install profile.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum Layout {
    /// The processors layout: `--installClient` runs the installer's own processors.
    Processors,
    /// The `versionInfo` layout, which has no client flag, so the loader installs it from the profile.
    VersionInfo,
}

/// The layout of the installer jar at `jar`. A jar with no install profile, or with processors, runs as an installer.
pub(crate) async fn layout(jar: PathBuf) -> Result<Layout, LoaderError> {
    let path = jar.clone();
    blocking(&path, move || {
        read_profile(&jar).map(|profile| match profile {
            Some(profile) if profile.get("processors").is_none() && profile.get("versionInfo").is_some() => Layout::VersionInfo,
            _ => Layout::Processors,
        })
    })
    .await
}

/// Installs a `versionInfo` profile into `game_dir`. The universal jar lands in `libraries/`, the libraries the window would
/// download land there too, and `versionInfo` is written as `versions/<target>/<target>.json`.
pub(crate) async fn install(
    http: &reqwest::Client,
    installer: &Path,
    game_dir: &Path,
    lines: Option<&Sender<String>>,
    progress: Option<&Sender<DownloadProgress>>,
) -> Result<InstalledVersion, LoaderError> {
    let libraries = game_dir.join("libraries");
    let owned = installer.to_path_buf();
    let profile = blocking(installer, move || {
        let profile = read_profile(&owned)?.ok_or_else(|| unreadable(&owned, "the installer has no install profile"))?;
        Profile::from_json(&owned, profile)
    })
    .await?;

    let extracted = {
        let (owned, libraries, profile) = (installer.to_path_buf(), libraries.clone(), profile.clone());
        blocking(installer, move || extract_universal(&owned, &profile, &libraries)).await?
    };
    say(lines, format!("Extracted {} to {}", profile.file_path, extracted.display())).await;

    let wanted = wanted_libraries(&profile.version_info);
    say(lines, format!("Downloading {} libraries", wanted.len())).await;
    let (libraries_dir, installer_path) = (libraries.as_path(), installer);
    let skipped: Vec<Option<String>> = futures_util::stream::iter(wanted)
        .map(|library| async move { fetch_library(http, installer_path, libraries_dir, &library, progress).await })
        .buffer_unordered(CONCURRENCY)
        .try_collect()
        .await?;
    for name in skipped.into_iter().flatten() {
        say(lines, format!("Skipped {name}: the launcher fetches it when the game starts")).await;
    }

    write_version(game_dir, &profile.target, &profile.version_info)
}

/// The parts of an install profile the install uses.
#[derive(Debug, Clone)]
struct Profile {
    /// The version the install writes: `install.target`, which names its folder under `versions/`.
    target: String,
    /// The maven coordinate of the universal jar: `install.path`.
    path: String,
    /// The universal jar's entry inside the installer: `install.filePath`.
    file_path: String,
    /// The version JSON: `versionInfo`, written as it is.
    version_info: Value,
}

impl Profile {
    fn from_json(jar: &Path, profile: Value) -> Result<Self, LoaderError> {
        let install = profile
            .get("install")
            .ok_or_else(|| unreadable(jar, "the profile has no install section"))?;
        let field = |name: &str| {
            install
                .get(name)
                .and_then(Value::as_str)
                .map(str::to_string)
                .ok_or_else(|| unreadable(jar, format!("the install section names no {name}")))
        };
        let version_info = profile
            .get("versionInfo")
            .cloned()
            .ok_or_else(|| unreadable(jar, "the profile has no versionInfo"))?;
        Ok(Self {
            target: field("target")?,
            path: field("path")?,
            file_path: field("filePath")?,
            version_info,
        })
    }
}

/// A library the window downloads: an entry of `versionInfo.libraries` marked `clientreq`.
#[derive(Debug, Clone)]
struct Library {
    name: String,
    url: Option<String>,
    checksums: Vec<String>,
}

/// The libraries the window downloads for the client. The window's optional libraries are not among them: those are opt-in.
fn wanted_libraries(version_info: &Value) -> Vec<Library> {
    version_info
        .get("libraries")
        .and_then(Value::as_array)
        .into_iter()
        .flatten()
        .filter(|entry| entry.get("clientreq").and_then(Value::as_bool) == Some(true))
        .filter_map(|entry| {
            Some(Library {
                name: entry.get("name")?.as_str()?.to_string(),
                url: entry.get("url").and_then(Value::as_str).map(str::to_string),
                checksums: entry
                    .get("checksums")
                    .and_then(Value::as_array)
                    .into_iter()
                    .flatten()
                    .filter_map(Value::as_str)
                    .map(str::to_string)
                    .collect(),
            })
        })
        .collect()
}

/// Gets one library into `libraries/`, as the window does. A library already there that matches its checksums is kept.
/// Otherwise it downloads from its `url`, and when that fails it takes the copy the installer carries under `maven/`. A failure
/// from the default library host is non-fatal and returns the library's name. Any other failure stops the install.
async fn fetch_library(
    http: &reqwest::Client,
    installer: &Path,
    libraries: &Path,
    library: &Library,
    progress: Option<&Sender<DownloadProgress>>,
) -> Result<Option<String>, LoaderError> {
    let relative =
        maven_jar_path(&library.name).ok_or_else(|| unreadable(installer, format!("the library {} is not a maven coordinate", library.name)))?;
    let local = libraries.join(&relative);
    if checksum_ok(local.clone(), library.checksums.clone()).await {
        return Ok(None);
    }
    // A file that is there but does not match would be kept by the downloader, so it goes first.
    let _ = std::fs::remove_file(&local);

    let base = library.url.as_deref().map_or_else(|| DEFAULT_LIBRARY_URL.to_string(), maven_base);
    let url = format!("{base}{relative}");
    let failure = match download_files(http, vec![Target::unchecked(url.clone(), local.clone())], progress).await {
        Ok(_) => {
            if checksum_ok(local.clone(), library.checksums.clone()).await {
                return Ok(None);
            }
            let _ = std::fs::remove_file(&local);
            LoaderError::Download {
                url: url.clone(),
                source: "the file does not match its listed checksum".into(),
            }
        }
        Err(error) => error,
    };
    if embedded_copy(installer.to_path_buf(), relative, local.clone(), library.checksums.clone()).await {
        return Ok(None);
    }
    if base.starts_with(DEFAULT_LIBRARY_URL) {
        return Ok(Some(library.name.clone()));
    }
    Err(failure)
}

/// Whether the file at `path` is there and matches one of `checksums`. A file is accepted when no checksum is listed. The hash
/// runs off the async runtime.
async fn checksum_ok(path: PathBuf, checksums: Vec<String>) -> bool {
    tokio::task::spawn_blocking(move || {
        if checksums.is_empty() {
            return path.is_file();
        }
        sha1_of_file(&path).is_ok_and(|actual| checksums.iter().any(|expected| expected.eq_ignore_ascii_case(&actual)))
    })
    .await
    .unwrap_or(false)
}

/// Copies a library from the installer's `maven/` folder to `local`, when the installer carries it and it matches its checksums.
/// Returns whether it did.
async fn embedded_copy(installer: PathBuf, relative: String, local: PathBuf, checksums: Vec<String>) -> bool {
    tokio::task::spawn_blocking(move || {
        let Ok(mut archive) = ZipManager::new(&installer) else {
            return false;
        };
        let Ok(mut entry) = archive.open_file(&format!("maven/{relative}")) else {
            return false;
        };
        let mut bytes = Vec::new();
        if entry.read_to_end(&mut bytes).is_err() || write_file(&local, &bytes).is_err() {
            return false;
        }
        let matches =
            checksums.is_empty() || sha1_of_file(&local).is_ok_and(|actual| checksums.iter().any(|expected| expected.eq_ignore_ascii_case(&actual)));
        if !matches {
            let _ = std::fs::remove_file(&local);
        }
        matches
    })
    .await
    .unwrap_or(false)
}

/// Copies the universal jar the profile names from the installer to the maven path `install.path` gives it under `libraries/`.
fn extract_universal(installer: &Path, profile: &Profile, libraries: &Path) -> Result<PathBuf, LoaderError> {
    let relative =
        maven_jar_path(&profile.path).ok_or_else(|| unreadable(installer, format!("install.path {} is not a maven coordinate", profile.path)))?;
    let local = libraries.join(relative);
    let mut archive = ZipManager::new(installer).map_err(|error| unreadable(installer, error))?;
    let mut entry = archive
        .open_file(&profile.file_path)
        .map_err(|error| unreadable(installer, format!("{} is not in the installer: {error}", profile.file_path)))?;
    let mut bytes = Vec::new();
    entry
        .read_to_end(&mut bytes)
        .map_err(|error| unreadable(installer, format!("{} could not be read: {error}", profile.file_path)))?;
    write_file(&local, &bytes)?;
    Ok(local)
}

/// The install profile inside the installer jar, or `None` when the jar has none.
fn read_profile(jar: &Path) -> Result<Option<Value>, LoaderError> {
    let mut archive = ZipManager::new(jar).map_err(|error| unreadable(jar, error))?;
    let Ok(text) = archive.read_to_string(PROFILE) else {
        return Ok(None);
    };
    serde_json::from_str(&text)
        .map(Some)
        .map_err(|error| unreadable(jar, format!("{PROFILE} is not JSON: {error}")))
}

/// Writes `bytes` to `path`, creating its folder first.
fn write_file(path: &Path, bytes: &[u8]) -> Result<(), LoaderError> {
    if let Some(parent) = path.parent() {
        std::fs::create_dir_all(parent).map_err(|source| LoaderError::Io {
            path: parent.to_path_buf(),
            source,
        })?;
    }
    lodestone_fs::write_atomic(path, bytes).map_err(|source| LoaderError::Io {
        path: path.to_path_buf(),
        source,
    })
}

/// Runs `work` on the blocking pool, so the zip reads and hashes it does stay off the async runtime.
async fn blocking<T: Send + 'static>(jar: &Path, work: impl FnOnce() -> Result<T, LoaderError> + Send + 'static) -> Result<T, LoaderError> {
    let jar = jar.to_path_buf();
    tokio::task::spawn_blocking(work)
        .await
        .map_err(|error| unreadable(&jar, format!("reading it stopped: {error}")))?
}

/// Tells the install's observer about a step, when it is listening.
async fn say(lines: Option<&Sender<String>>, line: String) {
    if let Some(lines) = lines {
        let _ = lines.send(line).await;
    }
}

fn unreadable(jar: &Path, detail: impl std::fmt::Display) -> LoaderError {
    LoaderError::InstallerUnreadable {
        loader: LOADER,
        path: jar.to_path_buf(),
        detail: detail.to_string(),
    }
}

#[cfg(test)]
#[path = "../tests/forge_profile.rs"]
mod tests;
