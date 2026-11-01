//! The loader stage. Fabric and Quilt write their profile and its libraries. Forge and NeoForge
//! download their installer and run it. Either way the loader lands in the shared store, and the
//! stage returns the version id the instance will start.

use std::path::Path;

use lodestone_loaders::{self as loaders, LoaderKind};
use tokio::sync::mpsc;

use super::ensure;
use super::java::system as system_java;
use crate::app::lang;
use crate::app::pages::library::instance::Loader;

/// Installs `loader_version` of `loader` for Minecraft `mc` into `shared`, and returns the id of
/// the version it wrote. `java` runs the Forge and NeoForge installers, and their output goes to
/// `lines` as it arrives.
pub(crate) async fn install(
    shared: &Path,
    mc: &str,
    loader: Loader,
    loader_version: &str,
    java: &Path,
    game_major: u32,
    lines: Option<&mpsc::Sender<String>>,
) -> Result<String, String> {
    match loader {
        Loader::Vanilla => Err(lang::t("install.loader.vanilla").to_owned()),
        Loader::Fabric => run::<loaders::Fabric>(shared, mc, loader_version, java, game_major, lines).await,
        Loader::Forge => run::<loaders::Forge>(shared, mc, loader_version, java, game_major, lines).await,
        Loader::Quilt => run::<loaders::Quilt>(shared, mc, loader_version, java, game_major, lines).await,
        Loader::NeoForge => run::<loaders::NeoForge>(shared, mc, loader_version, java, game_major, lines).await,
    }
}

/// Finds the build the modal chose among the loader's builds for `mc`, and installs it. An installer whose processors
/// write files that fail their checks runs once more on the system Java, when that is at least `game_major`.
async fn run<K: LoaderKind>(
    shared: &Path,
    mc: &str,
    loader_version: &str,
    java: &Path,
    game_major: u32,
    lines: Option<&mpsc::Sender<String>>,
) -> Result<String, String> {
    // Builds of one Minecraft version write the same processor outputs into the shared folder, so only one installer
    // runs at a time. The version lock keeps a download from replacing a file the installer reads.
    let _version = ensure::lock(format!("version:{mc}")).await;
    let _loader = ensure::lock("loader".to_string()).await;
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
    let installed = match handle.install(&version, shared, java, lines, None).await {
        Ok(installed) => installed,
        Err(error) if error.is_output_mismatch() => {
            let Some(system) = system_java(game_major).await else {
                return Err(error.to_string());
            };
            if let Some(lines) = lines {
                let _ = lines.send(lang::t("install.loader.system_java").to_owned()).await;
            }
            handle
                .install(&version, shared, &system.program, lines, None)
                .await
                .map_err(|error| error.to_string())?
        }
        Err(error) => return Err(error.to_string()),
    };
    Ok(installed.id)
}
