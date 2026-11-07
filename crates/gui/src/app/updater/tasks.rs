//! The update's work away from the UI thread: the download and its checksum, the unpacking, and the placing of the update.

use std::path::{Path, PathBuf};

use lodestone_updater::{
    Error, Payload, Release, extract_bundle, extract_executable, fetch_asset, relaunch_after_exit, replace_bundle, replace_executable,
    run_silent_elevated,
};

use super::{Target, USER_AGENT, Verified};
use crate::app::paths;

/// Where updates are staged: `updates` in the cache folder. One update is staged at a time, so a new download clears the
/// folder first.
fn staging() -> Result<PathBuf, String> {
    paths::cache_dir()
        .map(|dir| dir.join("updates"))
        .ok_or_else(|| "there is no cache folder to stage the update in".to_string())
}

/// Downloads the release's file, checks it against the release's checksums, and unpacks what the install uses.
pub async fn fetch(release: Release, payload: Payload) -> Result<Verified, String> {
    let root = staging()?;
    let clear = root.clone();
    let _ = tokio::task::spawn_blocking(move || std::fs::remove_dir_all(clear)).await;
    let client = lodestone_updater::client(USER_AGENT).map_err(|error| error.to_string())?;
    let name = payload.file_name(&release.tag);
    let dir = root.join(&release.tag);
    let archive = fetch_asset(&client, &release, &name, &dir).await.map_err(|error| error.to_string())?;
    let unpacked = dir.join("unpacked");
    let file = tokio::task::spawn_blocking(move || unpack(payload, &archive, &unpacked))
        .await
        .map_err(|error| error.to_string())??;
    Ok(Verified { release, payload: file })
}

/// The file the install uses, unpacked from the verified archive `payload` names. The Windows setup is the archive itself.
fn unpack(payload: Payload, archive: &Path, unpacked: &Path) -> Result<PathBuf, String> {
    match payload {
        Payload::WindowsInstaller => Ok(archive.to_path_buf()),
        Payload::LinuxArchive => extract_executable(archive, unpacked).map_err(|error| error.to_string()),
        Payload::MacosArchive => extract_bundle(archive, unpacked).map_err(|error| error.to_string()),
    }
}

/// Puts the update in place, on the blocking pool. The Windows setup runs silently, which closes the launcher and starts it
/// again. A Linux executable or a macOS app bundle is replaced, and the launcher restarts once this process has ended.
pub async fn apply(target: Target, payload: PathBuf) -> Result<(), String> {
    tokio::task::spawn_blocking(move || place(&target, &payload))
        .await
        .map_err(|error| error.to_string())?
}

fn place(target: &Target, payload: &Path) -> Result<(), String> {
    let pid = std::process::id();
    let outcome: Result<(), Error> = match target {
        Target::Installer => run_silent_elevated(payload),
        Target::Executable(executable) => replace_executable(payload, executable).and_then(|()| relaunch_after_exit(pid, executable, &[])),
        Target::Bundle(bundle) => {
            replace_bundle(payload, bundle).and_then(|()| relaunch_after_exit(pid, Path::new("/usr/bin/open"), &[bundle.as_path()]))
        }
        Target::Manual => Err(Error::Launch("this copy cannot install an update".to_string())),
    };
    outcome.map_err(|error| error.to_string())
}
