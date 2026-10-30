//! The loader stage. Fabric and Quilt write their profile and its libraries. Forge and
//! NeoForge download their installer and run it. Either way the install reports the
//! version id the instance will start.

use std::path::Path;

use lodestone_loaders::{self as loaders, DownloadProgress, LoaderKind};
use sipper::Sender as ProgressSender;
use tokio::sync::mpsc;

use super::{InstallProgress, InstallRequest, Stage, fraction};
use crate::app::lang;
use crate::app::pages::library::instance::Loader;

/// Installs the loader `request` asks for into `game_dir` and returns the version id the
/// instance starts. `java` runs the Forge and NeoForge installers.
pub(super) async fn install(
    request: &InstallRequest,
    game_dir: &Path,
    java: &Path,
    progress: &mut ProgressSender<InstallProgress>,
) -> Result<String, String> {
    let Some(loader_version) = request.loader_version.as_deref() else {
        return Err(lang::t("install.loader.choose_version").to_owned());
    };
    let mc = request.mc_version.as_str();
    match request.loader {
        Loader::Vanilla => Err(lang::t("install.loader.vanilla").to_owned()),
        Loader::Fabric => run::<loaders::Fabric>(mc, loader_version, game_dir, java, progress).await,
        Loader::Forge => run::<loaders::Forge>(mc, loader_version, game_dir, java, progress).await,
        Loader::Quilt => run::<loaders::Quilt>(mc, loader_version, game_dir, java, progress).await,
        Loader::NeoForge => run::<loaders::NeoForge>(mc, loader_version, game_dir, java, progress).await,
    }
}

/// Finds the build the modal chose among the loader's builds for `mc`, installs it, and
/// returns the id of the version it wrote.
async fn run<K: LoaderKind>(
    mc: &str,
    loader_version: &str,
    game_dir: &Path,
    java: &Path,
    progress: &mut ProgressSender<InstallProgress>,
) -> Result<String, String> {
    let handle = loaders::Loader::<K>::new();
    let version = handle
        .versions_for_minecraft(mc)
        .await
        .map_err(|error| error.to_string())?
        .into_iter()
        .find(|candidate| candidate.version == loader_version)
        .ok_or_else(|| {
            lang::t_with(
                "install.loader.unavailable",
                &[("loader", K::NAME), ("version", loader_version), ("mc", mc)],
            )
        })?;

    // The installer's output and the library downloads reach the dialog while the
    // install runs. The senders live until it finishes, so neither channel closes early.
    let (line_sender, mut lines) = mpsc::channel::<String>(256);
    let (download_sender, mut downloads) = mpsc::channel::<DownloadProgress>(64);
    let work = async { handle.install(&version, game_dir, java, Some(&line_sender), Some(&download_sender)).await };
    tokio::pin!(work);
    let installed = loop {
        tokio::select! {
            Some(line) = lines.recv() => {
                progress.send(InstallProgress { stage: Stage::Loader, fraction: None, detail: line }).await;
            }
            Some(report) = downloads.recv() => {
                let detail = lang::t("install.loader.libraries").to_owned();
                progress.send(InstallProgress {
                    stage: Stage::Loader,
                    fraction: fraction(report.bytes_downloaded, report.bytes_to_download),
                    detail,
                })
                .await;
            }
            output = &mut work => break output,
        }
    };
    installed.map(|installed| installed.id).map_err(|error| error.to_string())
}
