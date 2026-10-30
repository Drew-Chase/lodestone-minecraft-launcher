//! The loader stage. Fabric and Quilt write their profile and its libraries. Forge and NeoForge
//! download their installer and run it. Either way the loader lands in the shared store, and the
//! stage returns the version id the instance will start.

use std::path::Path;

use lodestone_loaders::{self as loaders, LoaderKind};
use tokio::sync::mpsc;

use super::ensure;
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
    lines: Option<&mpsc::Sender<String>>,
) -> Result<String, String> {
    match loader {
        Loader::Vanilla => Err(lang::t("install.loader.vanilla").to_owned()),
        Loader::Fabric => run::<loaders::Fabric>(shared, mc, loader_version, java, lines).await,
        Loader::Forge => run::<loaders::Forge>(shared, mc, loader_version, java, lines).await,
        Loader::Quilt => run::<loaders::Quilt>(shared, mc, loader_version, java, lines).await,
        Loader::NeoForge => run::<loaders::NeoForge>(shared, mc, loader_version, java, lines).await,
    }
}

/// Finds the build the modal chose among the loader's builds for `mc`, and installs it.
async fn run<K: LoaderKind>(
    shared: &Path,
    mc: &str,
    loader_version: &str,
    java: &Path,
    lines: Option<&mpsc::Sender<String>>,
) -> Result<String, String> {
    let _loader = ensure::lock(format!("loader:{}:{mc}:{loader_version}", K::NAME)).await;
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
    let installed = handle
        .install(&version, shared, java, lines, None)
        .await
        .map_err(|error| error.to_string())?;
    Ok(installed.id)
}
