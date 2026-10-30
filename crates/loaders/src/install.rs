//! The install steps the loader kinds share: writing a version JSON, seeding the
//! launcher profile the installers look for, and running an installer jar while its
//! output streams out (private, apart from [`InstalledVersion`]).

use std::collections::{BTreeSet, VecDeque};
use std::io::{BufRead, BufReader, Read};
use std::path::{Path, PathBuf};
use std::process::{Command, Stdio};
use std::thread::JoinHandle;
use std::time::Duration;

use tokio::sync::mpsc::Sender;

use crate::error::LoaderError;

/// A version an install produced: its id and the version JSON on disk.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct InstalledVersion {
    /// The version id, which also names its folder under `versions/`.
    pub id: String,
    /// The version JSON: `versions/<id>/<id>.json` under the game folder.
    pub json: PathBuf,
}

/// The installer output lines kept for an error message.
const TAIL_LINES: usize = 40;

/// How often a running installer is checked for a dropped line receiver.
const POLL: Duration = Duration::from_millis(250);

/// Writes `json` as `versions/<id>/<id>.json` under `game_dir`.
pub(crate) fn write_version(game_dir: &Path, id: &str, json: &serde_json::Value) -> Result<InstalledVersion, LoaderError> {
    let directory = game_dir.join("versions").join(id);
    std::fs::create_dir_all(&directory).map_err(|source| LoaderError::Io {
        path: directory.clone(),
        source,
    })?;
    let path = directory.join(format!("{id}.json"));
    std::fs::write(&path, json.to_string()).map_err(|source| LoaderError::Io { path: path.clone(), source })?;
    Ok(InstalledVersion {
        id: id.to_string(),
        json: path,
    })
}

/// Writes an empty launcher profile into `game_dir` unless one is there already.
/// The Forge and NeoForge installers refuse to run without it.
pub(crate) fn seed_launcher_profiles(game_dir: &Path) -> Result<(), LoaderError> {
    std::fs::create_dir_all(game_dir).map_err(|source| LoaderError::Io {
        path: game_dir.to_path_buf(),
        source,
    })?;
    let path = game_dir.join("launcher_profiles.json");
    if !path.exists() {
        std::fs::write(&path, r#"{"profiles":{}}"#).map_err(|source| LoaderError::Io { path: path.clone(), source })?;
    }
    Ok(())
}

/// The installer jar among the jars a Forge or NeoForge `download_client` fetched.
pub(crate) fn installer_jar(jars: &[PathBuf]) -> Option<&PathBuf> {
    jars.iter()
        .find(|jar| jar.file_name().is_some_and(|name| name.to_string_lossy().ends_with("-installer.jar")))
}

/// The marker a finished installer run leaves under `game_dir/installs`, named for the
/// loader build. It holds the id of the version the run wrote, so the next install of the
/// same build reads it back and skips the installer.
pub(crate) fn installer_marker(game_dir: &Path, loader: &str, minecraft: &str, version: &str) -> PathBuf {
    game_dir.join("installs").join(format!("{loader}-{minecraft}-{version}.id"))
}

/// The version a finished install recorded, when its version JSON is still on disk.
pub(crate) fn finished_install(game_dir: &Path, marker: &Path) -> Option<InstalledVersion> {
    let id = std::fs::read_to_string(marker).ok()?.trim().to_string();
    let json = game_dir.join("versions").join(&id).join(format!("{id}.json"));
    json.is_file().then_some(InstalledVersion { id, json })
}

/// Records a finished install so the next one of the same build can skip the installer.
pub(crate) fn record_install(marker: &Path, installed: &InstalledVersion) -> Result<(), LoaderError> {
    if let Some(parent) = marker.parent() {
        std::fs::create_dir_all(parent).map_err(|source| LoaderError::Io {
            path: parent.to_path_buf(),
            source,
        })?;
    }
    std::fs::write(marker, &installed.id).map_err(|source| LoaderError::Io {
        path: marker.to_path_buf(),
        source,
    })
}

/// The folder names under `game_dir/versions`. A missing folder counts as empty.
fn version_folders(game_dir: &Path) -> Result<BTreeSet<String>, LoaderError> {
    let versions = game_dir.join("versions");
    let mut names = BTreeSet::new();
    if !versions.exists() {
        return Ok(names);
    }
    let entries = std::fs::read_dir(&versions).map_err(|source| LoaderError::Io {
        path: versions.clone(),
        source,
    })?;
    for entry in entries {
        let entry = entry.map_err(|source| LoaderError::Io {
            path: versions.clone(),
            source,
        })?;
        if entry.path().is_dir() {
            names.insert(entry.file_name().to_string_lossy().into_owned());
        }
    }
    Ok(names)
}

/// Runs `java -jar <installer> <flag> <game_dir>` in `spawn_blocking` and returns the
/// loader's version, the new folder whose JSON inherits from `minecraft`.
pub(crate) async fn run_installer_task(
    loader: &'static str,
    minecraft: &str,
    java: &Path,
    installer: &Path,
    flag: &'static str,
    game_dir: &Path,
    lines: Option<&Sender<String>>,
) -> Result<InstalledVersion, LoaderError> {
    let (minecraft, java, installer, game_dir) = (minecraft.to_string(), java.to_path_buf(), installer.to_path_buf(), game_dir.to_path_buf());
    let lines = lines.cloned();
    tokio::task::spawn_blocking(move || run_installer(loader, &minecraft, java, installer, flag, game_dir, lines))
        .await
        .map_err(|error| LoaderError::InstallerFailed {
            loader,
            status: "stopped".to_string(),
            output: error.to_string(),
        })?
}

/// The blocking half of [`run_installer_task`]. Output lines go to `lines` as they
/// arrive. When that receiver is dropped the installer is stopped and the run fails.
fn run_installer(
    loader: &'static str,
    minecraft: &str,
    java: PathBuf,
    installer: PathBuf,
    flag: &'static str,
    game_dir: PathBuf,
    lines: Option<Sender<String>>,
) -> Result<InstalledVersion, LoaderError> {
    let before = version_folders(&game_dir)?;
    let mut child = Command::new(&java)
        .arg("-jar")
        .arg(&installer)
        .arg(flag)
        .arg(&game_dir)
        .current_dir(&game_dir)
        .stdin(Stdio::null())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .map_err(|source| LoaderError::InstallerLaunch {
            program: java.clone(),
            source,
        })?;
    let stdout = child.stdout.take().map(|pipe| read_lines(pipe, lines.clone()));
    let stderr = child.stderr.take().map(|pipe| read_lines(pipe, lines.clone()));

    let status = loop {
        match child.try_wait() {
            Ok(Some(status)) => break Some(status),
            Ok(None) => {}
            Err(_) => break None,
        }
        if lines.as_ref().is_some_and(Sender::is_closed) {
            let _ = child.kill();
            let _ = child.wait();
            break None;
        }
        std::thread::sleep(POLL);
    };

    let mut output: Vec<String> = Vec::new();
    for reader in [stdout, stderr].into_iter().flatten() {
        output.extend(reader.join().unwrap_or_default());
    }
    let tail = output[output.len().saturating_sub(TAIL_LINES)..].join("\n");
    if !status.is_some_and(|status| status.success()) {
        let status = match status {
            Some(status) => match status.code() {
                Some(code) => format!("exit code {code}"),
                None => "terminated by a signal".to_string(),
            },
            None => "stopped".to_string(),
        };
        return Err(LoaderError::InstallerFailed {
            loader,
            status,
            output: tail,
        });
    }

    // The installer also writes the vanilla version it builds on, which has no `inheritsFrom`.
    // The loader's version is the new folder whose JSON inherits from `minecraft`.
    let after = version_folders(&game_dir)?;
    for id in after.difference(&before) {
        let json = game_dir.join("versions").join(id).join(format!("{id}.json"));
        if inherits_from(&json).as_deref() == Some(minecraft) {
            return Ok(InstalledVersion { id: id.clone(), json });
        }
    }
    Err(LoaderError::InstallerVersionMissing {
        loader,
        directory: game_dir.join("versions"),
    })
}

/// The `inheritsFrom` of the version JSON at `path`, when it has one.
fn inherits_from(path: &Path) -> Option<String> {
    let text = std::fs::read_to_string(path).ok()?;
    let json: serde_json::Value = serde_json::from_str(&text).ok()?;
    json.get("inheritsFrom")?.as_str().map(str::to_string)
}

/// Reads `pipe` line by line on its own thread, forwards each line to `lines`, and
/// returns the last [`TAIL_LINES`] lines when the pipe closes.
fn read_lines<R: Read + Send + 'static>(pipe: R, lines: Option<Sender<String>>) -> JoinHandle<Vec<String>> {
    std::thread::spawn(move || {
        let mut tail: VecDeque<String> = VecDeque::with_capacity(TAIL_LINES);
        for line in BufReader::new(pipe).lines() {
            let Ok(line) = line else { break };
            if let Some(sender) = &lines {
                let _ = sender.blocking_send(line.clone());
            }
            if tail.len() == TAIL_LINES {
                tail.pop_front();
            }
            tail.push_back(line);
        }
        Vec::from(tail)
    })
}

#[cfg(test)]
#[path = "../tests/install.rs"]
mod tests;
