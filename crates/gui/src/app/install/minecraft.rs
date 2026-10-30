//! The Minecraft files an instance needs: its version JSON and client jar, the libraries
//! the version lists, and the asset index and objects, which are shared per version.

use std::path::Path;

use piston_mc::manifest_v2::ManifestV2;
use piston_mc::version_manifest::{LibraryItemDownloader, VersionManifest};
use simple_download_utility::{DownloadProgress, MultiDownloadProgress};
use sipper::Sender as ProgressSender;
use tokio::sync::mpsc;

use super::{COMPLETE, InstallProgress, PARALLEL, Stage, drive, fraction};
use crate::app::lang;
use crate::app::paths::LauncherPaths;

/// A version as Mojang lists it: parsed, with the JSON text it came from, and the Java
/// runtime component it asks for.
pub(super) struct Resolved {
    pub(super) version: VersionManifest,
    pub(super) raw: String,
    pub(super) component: String,
}

/// Looks `mc` up in Mojang's version list and downloads its version JSON.
pub(super) async fn resolve(mc: &str) -> Result<Resolved, String> {
    let manifest = ManifestV2::fetch()
        .await
        .map_err(|error| lang::t_with("install.version_list_failed", &[("error", error.to_string().as_str())]))?;
    let entry = manifest
        .versions
        .iter()
        .find(|entry| entry.id == mc)
        .ok_or_else(|| lang::t_with("install.version_missing", &[("mc", mc)]))?;
    let raw = reqwest::get(&entry.url)
        .await
        .and_then(reqwest::Response::error_for_status)
        .map_err(|error| lang::t_with("install.version_fetch_failed", &[("mc", mc), ("error", error.to_string().as_str())]))?
        .text()
        .await
        .map_err(|error| lang::t_with("install.version_read_failed", &[("mc", mc), ("error", error.to_string().as_str())]))?;
    let version: VersionManifest = serde_json::from_str(&raw)
        .map_err(|error| lang::t_with("install.version_parse_failed", &[("mc", mc), ("error", error.to_string().as_str())]))?;
    let component = version
        .java_version
        .as_ref()
        .map_or_else(|| "jre-legacy".to_string(), |java| java.component.clone());
    Ok(Resolved { version, raw, component })
}

/// Writes the version JSON as `versions/<mc>/<mc>.json`, the file the Forge and NeoForge
/// installers read.
pub(super) fn write_version_json(game_dir: &Path, mc: &str, raw: &str) -> Result<(), String> {
    let dir = game_dir.join("versions").join(mc);
    std::fs::create_dir_all(&dir).map_err(|error| super::could_not_create(&dir, error))?;
    let path = dir.join(format!("{mc}.json"));
    std::fs::write(&path, raw).map_err(|error| super::path_failed("install.could_not_write", &path, error))
}

/// Downloads and checks the client jar into `versions/<mc>/<mc>.jar`.
pub(super) async fn download_client(
    version: &VersionManifest,
    game_dir: &Path,
    progress: &mut ProgressSender<InstallProgress>,
) -> Result<(), String> {
    let jar = game_dir.join("versions").join(&version.id).join(format!("{}.jar", version.id));
    let (sender, reports) = mpsc::channel::<DownloadProgress>(64);
    let work = async { version.download_client(&jar, true, Some(sender)).await.map_err(|error| error.to_string()) };
    drive(
        progress,
        reports,
        |report: DownloadProgress| InstallProgress {
            stage: Stage::Client,
            fraction: fraction(report.bytes_downloaded, report.bytes_to_download),
            detail: lang::t_with("install.client_progress", &[("id", version.id.as_str())]),
        },
        work,
    )
    .await
}

/// Downloads the version's libraries into `libraries/`, natives included.
pub(super) async fn download_libraries(
    version: &VersionManifest,
    game_dir: &Path,
    progress: &mut ProgressSender<InstallProgress>,
) -> Result<(), String> {
    let (sender, reports) = mpsc::channel::<MultiDownloadProgress>(64);
    let libraries = game_dir.join("libraries");
    let work = async {
        version
            .libraries
            .download(&libraries, PARALLEL, Some(sender))
            .await
            .map(|_| ())
            .map_err(|error| error.to_string())
    };
    drive(progress, reports, multi_progress(Stage::Libraries), work).await
}

/// Downloads the asset index and every object into `meta/assets/<mc>`. A finished copy is
/// shared by every instance of that version, so it is not downloaded twice.
pub(super) async fn download_assets(
    version: &VersionManifest,
    paths: &LauncherPaths,
    progress: &mut ProgressSender<InstallProgress>,
) -> Result<(), String> {
    let folder = paths.assets(&version.id);
    if folder.join(COMPLETE).is_file() {
        super::report(
            progress,
            Stage::Assets,
            Some(1.0),
            lang::t_with("install.already_downloaded", &[("id", version.id.as_str())]),
        )
        .await;
        return Ok(());
    }
    let mut assets = version.assets().await.map_err(|error| {
        lang::t_with(
            "install.asset_index_failed",
            &[("id", version.id.as_str()), ("error", error.to_string().as_str())],
        )
    })?;
    let (sender, reports) = mpsc::channel::<MultiDownloadProgress>(64);
    let work = async { assets.download(&folder, PARALLEL, Some(sender)).await.map_err(|error| error.to_string()) };
    drive(progress, reports, multi_progress(Stage::Assets), work).await?;
    std::fs::write(folder.join(COMPLETE), b"").map_err(|error| super::path_failed("install.could_not_finish", &folder, error))
}

/// Turns a multi-file download report into install progress for `stage`.
fn multi_progress(stage: Stage) -> impl Fn(MultiDownloadProgress) -> InstallProgress {
    move |report| InstallProgress {
        stage,
        fraction: fraction(report.files_downloaded, report.files_total),
        detail: lang::t_with(
            "install.files_count",
            &[
                ("done", report.files_downloaded.to_string().as_str()),
                ("total", report.files_total.to_string().as_str()),
            ],
        ),
    }
}

#[cfg(test)]
#[path = "../../../tests/app/install/minecraft.rs"]
mod tests;
