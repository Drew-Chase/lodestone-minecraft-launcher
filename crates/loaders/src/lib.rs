//! # lodestone_loaders
//!
//! One crate, one struct, every loader: [`Loader`] parameterized by a marker kind —
//! [`Fabric`], [`Forge`], [`NeoForge`], [`Quilt`], [`Vanilla`] — all with the same
//! function surface.
//!
//! ```no_run
//! # async fn demo() -> Result<(), lodestone_loaders::LoaderError> {
//! use lodestone_loaders::{Loader, Quilt};
//!
//! let loader = Loader::<Quilt>::new();
//! let versions = loader.versions_for_minecraft("1.21.1").await?;
//! let latest = versions.first().expect("quilt has 1.21.1 builds");
//! let files = loader.download_client(latest, "instance-dir/libraries", None).await?;
//! let info = loader.extract_mod_info("mods/my-mod.jar")?;
//! println!("{} {} by {:?}", info.id, info.version.as_deref().unwrap_or("?"), info.authors);
//! # Ok(())
//! # }
//! ```
//!
//! ## Hard rules
//!
//! - **Standalone leaf crate.** Zero path dependencies on workspace siblings; only
//!   `crates/gui` links to workspace crates.
//! - **Async, tokio-based** — iced `Task`s await it directly.
//! - **Dumb about instances, smart about loader ecosystems.** It never writes instance
//!   profiles, never launches the game, and never talks to content platforms.
//! - **Wheels are not reinvented**: vanilla goes through `piston-mc`, downloads go
//!   through `simple_download_utility`, and mod metadata goes through `modcrawl` — see
//!   the wheel audit in `DESIGN.md`. Only the loader meta-API bindings and
//!   `quilt.mod.json` are implemented here, because nothing upstream covers them.
//!
//! ## What this crate does
//!
//! - List a loader's versions (`list_versions`)
//! - Get versions for a Minecraft version (`versions_for_minecraft`)
//! - Download a loader version's client libraries (`download_client`)
//! - Extract mod information from a jar (`extract_mod_info`)
//!
//! ## What this crate does NOT do
//!
//! - Launch the game (see `lodestone_launch`)
//! - Install loader profiles / merge version JSONs (follow-up slice)
//! - Download or manage mods (see the platform crates)
//! - Authenticate (see `auth`)

#![deny(missing_docs)]
#![forbid(unsafe_code)]

mod error;
mod fabric;
mod forge;
mod http;
mod meta;
mod mod_info;
mod neoforge;
mod quilt;
mod vanilla;

pub use error::LoaderError;
pub use fabric::Fabric;
pub use forge::Forge;
pub use meta::{LoaderVersion, ModDependency, ModDependencyKind, ModInfo};
pub use neoforge::NeoForge;
pub use quilt::Quilt;
pub use simple_download_utility::DownloadProgress;
pub use vanilla::Vanilla;

use std::future::Future;
use std::marker::PhantomData;
use std::path::{Path, PathBuf};
use tokio::sync::mpsc::Sender;

/// Sealed: the five marker types in this crate are the only implementations.
mod sealed {
    pub trait Sealed {}
}

/// Per-kind behavior behind the uniform [`Loader`] surface.
///
/// Sealed on purpose: adding a loader is a change to this crate (new submodule, new
/// marker, new `impl`), never to downstream code — downstream only ever names
/// `Loader<SomeKind>`.
pub trait LoaderKind: sealed::Sealed + Send + Sync + 'static {
    /// Lowercase loader name, e.g. `"fabric"`.
    const NAME: &'static str;
    /// Whether jars of this ecosystem carry client-mod metadata.
    const SUPPORTS_MOD_INFO: bool;

    /// Every version the loader's source currently lists.
    fn list_versions(http: &reqwest::Client) -> impl Future<Output = Result<Vec<LoaderVersion>, LoaderError>> + Send;

    /// The loader versions available for one Minecraft version.
    fn versions_for_minecraft(http: &reqwest::Client, minecraft: &str) -> impl Future<Output = Result<Vec<LoaderVersion>, LoaderError>> + Send;

    /// Downloads the loader version's client-side libraries into `into` and returns
    /// their paths. Progress events flow through `progress` when given.
    fn download_client(
        http: &reqwest::Client,
        version: &LoaderVersion,
        into: &Path,
        progress: Option<&Sender<DownloadProgress>>,
    ) -> impl Future<Output = Result<Vec<PathBuf>, LoaderError>> + Send;

    /// Extracts mod metadata from a jar file. The detected format wins:
    /// `ModInfo.loader` reports what was actually found, never the requested kind.
    fn extract_mod_info(jar: &Path) -> Result<ModInfo, LoaderError>;
}

/// The single behavioral struct of this crate: a loader handle for one kind.
///
/// The kind marker is a zero-sized type ([`Fabric`], [`Forge`], [`NeoForge`], [`Quilt`],
/// [`Vanilla`]); every kind shares the identical function surface. Constructing,
/// cloning, and storing the handle is cheap — the [`reqwest::Client`] inside is
/// connection-pooled and meant to be reused.
///
/// Launching the same kind from several handles is legal and stateless: no globals,
/// no statics, no interior mutability.
pub struct Loader<K: LoaderKind> {
    http: reqwest::Client,
    kind: PhantomData<fn() -> K>,
}

impl<K: LoaderKind> Default for Loader<K> {
    fn default() -> Self {
        Self::new()
    }
}

impl<K: LoaderKind> Loader<K> {
    /// Creates a loader handle with a default [`reqwest::Client`].
    pub fn new() -> Self {
        Self {
            http: reqwest::Client::new(),
            kind: PhantomData,
        }
    }

    /// Creates a loader handle around a preconfigured client (timeouts, proxy, …).
    pub fn with_client(http: reqwest::Client) -> Self {
        Self { http, kind: PhantomData }
    }

    /// Every version the loader's source currently lists.
    pub async fn list_versions(&self) -> Result<Vec<LoaderVersion>, LoaderError> {
        K::list_versions(&self.http).await
    }

    /// The loader versions available for one Minecraft version.
    pub async fn versions_for_minecraft(&self, minecraft: &str) -> Result<Vec<LoaderVersion>, LoaderError> {
        K::versions_for_minecraft(&self.http, minecraft).await
    }

    /// Downloads the loader version's client-side libraries into `into` and returns
    /// their paths. Progress events flow through `progress` when given.
    ///
    /// Versions obtained from [`Loader::list_versions`] carry no Minecraft binding for
    /// most kinds; fetch them with [`Loader::versions_for_minecraft`] first or this
    /// fails with [`LoaderError::MinecraftVersionUnknown`].
    pub async fn download_client(
        &self,
        version: &LoaderVersion,
        into: impl AsRef<Path>,
        progress: Option<&Sender<DownloadProgress>>,
    ) -> Result<Vec<PathBuf>, LoaderError> {
        K::download_client(&self.http, version, into.as_ref(), progress).await
    }

    /// Extracts mod metadata from a jar file.
    ///
    /// The detected format wins: [`ModInfo::loader`] reports what was actually found,
    /// never the requested kind — this marker is a convenience, not a filter.
    pub fn extract_mod_info(&self, jar: impl AsRef<Path>) -> Result<ModInfo, LoaderError> {
        if !K::SUPPORTS_MOD_INFO {
            return Err(LoaderError::ModInfoUnsupported { loader: K::NAME });
        }
        K::extract_mod_info(jar.as_ref())
    }
}
