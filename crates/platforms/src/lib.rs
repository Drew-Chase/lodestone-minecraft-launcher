//! # lodestone_platforms
//!
//! One crate, one struct, every content platform: [`Platform`] parameterized by a
//! marker kind — [`Modrinth`], [`CurseForge`], [`AtLauncher`], [`Ftb`],
//! [`Technic`] — all with the same function surface.
//!
//! ```no_run
//! # async fn demo() -> Result<(), lodestone_platforms::PlatformError> {
//! use lodestone_platforms::{ContentType, Platform, SearchQuery};
//!
//! let platform = Platform::<lodestone_platforms::Modrinth>::new();
//! let query = SearchQuery {
//!     text: "sodium".to_string(),
//!     ..SearchQuery::new(ContentType::Mod)
//! };
//! let page = platform.search(&query).await?;
//! let first = page.hits.first().expect("modrinth has sodium");
//! let details = platform.details(&first.id, ContentType::Mod).await?;
//! let versions = platform.versions(&first.id, ContentType::Mod).await?;
//! let latest = versions.first().expect("sodium has releases");
//! let files = platform.download(latest, "downloads", None).await?;
//! # Ok(())
//! # }
//! ```
//!
//! ## Hard rules
//!
//! - **Standalone leaf crate.** Zero path dependencies on workspace siblings; only
//!   `crates/gui` links to workspace crates.
//! - **Async, tokio-based** — iced `Task`s await it directly.
//! - **Dumb about instances.** It never installs content, never launches, never
//!   resolves loaders; downloaded files and parsed modpack data are the caller's to
//!   place.
//! - **Wheels are not reinvented**: Modrinth goes through `ferinth`; downloads go
//!   through `simple_download_utility`; archive reading goes through `zipcrawl`.
//!   CurseForge, ATLauncher, FTB, and Technic are thin bindings — no maintained
//!   crates cover them (see the wheel audit in `DESIGN.md`).
//! - **Capabilities are declared, not discovered**: each kind lists its
//!   [`ContentType`]s and whether it has a modpack archive format; out-of-capability
//!   requests fail fast with [`PlatformError::UnsupportedContentType`] or
//!   [`PlatformError::UnsupportedPlatform`].
//!
//! ## What this crate does
//!
//! - Search content by query and content type (`search`)
//! - Fetch project/pack details (`details`)
//! - List versions — mod file sets or pack builds (`versions`)
//! - Download a version's files (`download`)
//! - Parse modpack archives into installable data (`parse_modpack`)
//!
//! ## What this crate does NOT do
//!
//! - Install content into instances (callers decide where files go)
//! - Provide the unified cross-platform content browser (gui's job)
//! - Launch the game (see `lodestone_launch`) or resolve loaders
//!   (see `lodestone_loaders`)

#![deny(missing_docs)]
#![forbid(unsafe_code)]

mod atlauncher;
mod curseforge;
mod error;
mod ftb;
mod http;
mod meta;
mod modpack;
mod modrinth;
mod technic;

pub use atlauncher::AtLauncher;
pub use curseforge::CurseForge;
pub use error::PlatformError;
pub use ftb::Ftb;
pub use meta::{
    ContentDetails, ContentFile, ContentType, ContentVersion, Facet, Facets, Modpack, ModpackArchive, ModpackFile, SearchPage, SearchQuery,
    SearchResult, SearchSort,
};
pub use modrinth::Modrinth;
pub use simple_download_utility::DownloadProgress;
pub use technic::Technic;

use std::future::Future;
use std::marker::PhantomData;
use std::path::{Path, PathBuf};
use tokio::sync::mpsc::Sender;

/// Sealed: the five marker types in this crate are the only implementations.
mod sealed {
    pub trait Sealed {}
}

/// Per-kind behavior behind the uniform [`Platform`] surface.
///
/// Sealed on purpose: adding a platform is a change to this crate (new submodule,
/// new marker, new `impl`), never to downstream code — downstream only ever names
/// `Platform<SomeKind>`.
///
/// `credential` carries platform secrets (CurseForge's API key); kinds that need
/// none ignore it.
pub trait PlatformKind: sealed::Sealed + Send + Sync + 'static {
    /// Lowercase platform name, e.g. `"modrinth"`.
    const NAME: &'static str;
    /// The content types this platform hosts; requests outside this list are
    /// rejected before any network call.
    const CONTENT_TYPES: &'static [ContentType];
    /// Whether this platform has a modpack archive format `parse_modpack` can
    /// read (`.mrpack`, CurseForge exports).
    const SUPPORTS_MODPACK_ARCHIVE: bool;

    /// Searches the platform: one page of hits matching `query`.
    fn search(
        http: &reqwest::Client,
        credential: Option<&str>,
        query: &SearchQuery,
    ) -> impl Future<Output = Result<SearchPage, PlatformError>> + Send;

    /// The filters the platform offers for `content`.
    fn facets(http: &reqwest::Client, credential: Option<&str>, content: ContentType) -> impl Future<Output = Result<Facets, PlatformError>> + Send;

    /// Fetches the full details of one project or pack.
    fn details(
        http: &reqwest::Client,
        credential: Option<&str>,
        id: &str,
        content: ContentType,
    ) -> impl Future<Output = Result<ContentDetails, PlatformError>> + Send;

    /// Lists the versions (mod file sets / pack builds) of one project or pack.
    fn versions(
        http: &reqwest::Client,
        credential: Option<&str>,
        id: &str,
        content: ContentType,
    ) -> impl Future<Output = Result<Vec<ContentVersion>, PlatformError>> + Send;

    /// Downloads a version's files into `into` and returns their paths.
    /// Progress events flow through `progress` when given.
    fn download(
        http: &reqwest::Client,
        credential: Option<&str>,
        version: &ContentVersion,
        into: &Path,
        progress: Option<&Sender<DownloadProgress>>,
    ) -> impl Future<Output = Result<Vec<PathBuf>, PlatformError>> + Send;

    /// Parses a modpack archive into installable modpack data.
    fn parse_modpack(archive: &Path) -> Result<Modpack, PlatformError>;
}

/// The single behavioral struct of this crate: a platform handle for one kind.
///
/// The kind marker is a zero-sized type ([`Modrinth`], [`CurseForge`],
/// [`AtLauncher`], [`Ftb`], [`Technic`]); every kind shares the identical function
/// surface. Constructing, cloning, and storing the handle is cheap — the
/// [`reqwest::Client`] inside is connection-pooled and meant to be reused.
///
/// Launching the same kind from several handles is legal and stateless: a handle
/// holds no state of its own. The one shared cache is the answer cache of the
/// GET helper, which keeps repeated reads for ten minutes so the platforms'
/// rate limits are not spent on them again. The Modrinth kind's ferinth client
/// is an immutable shared instance.
pub struct Platform<K: PlatformKind> {
    http: reqwest::Client,
    credential: Option<String>,
    kind: PhantomData<fn() -> K>,
}

impl<K: PlatformKind> Platform<K> {
    fn build(credential: Option<String>, http: reqwest::Client) -> Self {
        Self {
            http,
            credential,
            kind: PhantomData,
        }
    }

    fn gate(&self, content: ContentType) -> Result<(), PlatformError> {
        if K::CONTENT_TYPES.contains(&content) {
            Ok(())
        } else {
            Err(PlatformError::UnsupportedContentType { platform: K::NAME, content })
        }
    }

    /// Creates a platform handle with a default [`reqwest::Client`].
    ///
    /// For [`CurseForge`] this creates a keyless handle: every call will fail
    /// with an authorization error until [`Platform::<CurseForge>::with_api_key`]
    /// is used instead.
    pub fn new() -> Self {
        Self::build(None, http::client())
    }

    /// Creates a platform handle around a preconfigured client (timeouts,
    /// proxy, …).
    ///
    /// Note: the Modrinth kind constructs its own client inside ferinth, so the
    /// injected client does not reach it.
    pub fn with_client(http: reqwest::Client) -> Self {
        Self::build(None, http)
    }

    /// Searches the platform: one page of hits matching `query`.
    pub async fn search(&self, query: &SearchQuery) -> Result<SearchPage, PlatformError> {
        self.gate(query.content)?;
        K::search(&self.http, self.credential.as_deref(), query).await
    }

    /// The filters the platform offers for `content`.
    pub async fn facets(&self, content: ContentType) -> Result<Facets, PlatformError> {
        self.gate(content)?;
        K::facets(&self.http, self.credential.as_deref(), content).await
    }

    /// Fetches the full details of one project or pack.
    pub async fn details(&self, id: &str, content: ContentType) -> Result<ContentDetails, PlatformError> {
        self.gate(content)?;
        K::details(&self.http, self.credential.as_deref(), id, content).await
    }

    /// Lists the versions (mod file sets / pack builds) of one project or pack.
    pub async fn versions(&self, id: &str, content: ContentType) -> Result<Vec<ContentVersion>, PlatformError> {
        self.gate(content)?;
        K::versions(&self.http, self.credential.as_deref(), id, content).await
    }

    /// Downloads a version's files into `into` and returns their paths.
    /// Progress events flow through `progress` when given.
    pub async fn download(
        &self,
        version: &ContentVersion,
        into: impl AsRef<Path>,
        progress: Option<&Sender<DownloadProgress>>,
    ) -> Result<Vec<PathBuf>, PlatformError> {
        let content = version.content_type;
        self.gate(content)?;
        K::download(&self.http, self.credential.as_deref(), version, into.as_ref(), progress).await
    }

    /// Parses a modpack archive into installable modpack data.
    ///
    /// Only platforms with an archive format support this (`.mrpack` for
    /// Modrinth, CurseForge export zips); the others return
    /// [`PlatformError::UnsupportedPlatform`] — their mod lists are server-side.
    pub fn parse_modpack(&self, archive: impl AsRef<Path>) -> Result<Modpack, PlatformError> {
        if !K::SUPPORTS_MODPACK_ARCHIVE {
            return Err(PlatformError::UnsupportedPlatform {
                platform: K::NAME,
                action: "modpack archive parsing",
            });
        }
        K::parse_modpack(archive.as_ref())
    }
}

impl<K: PlatformKind> Default for Platform<K> {
    fn default() -> Self {
        Self::new()
    }
}

/// CurseForge requires an API key (https://console.curseforge.com): every call
/// carries it as the `x-api-key` header. The keyless `new()` compiles for
/// CurseForge too, but every call will fail with an authorization error.
impl Platform<CurseForge> {
    /// Creates a CurseForge handle around an API key.
    pub fn with_api_key(key: impl Into<String>) -> Self {
        Self::build(Some(key.into()), http::client())
    }

    /// The CDN url of one file of a project, which a modpack's file list names by id. Fails when the
    /// file's author disabled distribution.
    pub async fn file_url(&self, project_id: &str, file_id: &str) -> Result<String, PlatformError> {
        curseforge::file_url(self.credential.as_deref(), project_id, file_id).await
    }
}

impl Platform<Ftb> {
    /// The installable modpack of one FTB version: its files at their install paths on FTB's CDN, and
    /// the Minecraft version, loader and loader version it targets. The version must come from
    /// [`Platform::versions`], which names its pack.
    pub async fn modpack(&self, version: &ContentVersion) -> Result<Modpack, PlatformError> {
        ftb::modpack(&self.http, version).await
    }
}

impl Platform<Technic> {
    /// The installable modpack of one Solder build: the mods its manifest names, and the Minecraft and
    /// Forge versions it targets. A zip pack is read from its downloaded zip by `parse_modpack`. The
    /// version must come from [`Platform::versions`], which names its pack.
    pub async fn modpack(&self, version: &ContentVersion) -> Result<Modpack, PlatformError> {
        technic::modpack(&self.http, version).await
    }
}

impl Platform<AtLauncher> {
    /// The installable modpack of one ATLauncher pack version, from its `Configs.json` on the CDN. The
    /// version must come from [`Platform::versions`], which names its pack.
    pub async fn modpack(&self, version: &ContentVersion) -> Result<Modpack, PlatformError> {
        atlauncher::modpack(&self.http, version).await
    }
}
