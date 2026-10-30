//! Errors surfaced by resolving, downloading, and analyzing loaders.

use std::path::PathBuf;

/// Everything that can go wrong in this crate. Messages are written to be shown to
/// a user verbatim: gui surfaces them without extra formatting, and each variant
/// carries the context needed to explain itself — the offending url, path, or
/// version pair.
#[derive(Debug, thiserror::Error)]
pub enum LoaderError {
    /// A meta API could not be reached at all.
    #[error("failed to reach {url}: {source}")]
    Network {
        /// The url that was being fetched.
        url: String,
        /// The underlying transport failure.
        #[source]
        source: reqwest::Error,
    },

    /// A meta API answered, but with an unexpected HTTP status.
    #[error("{url} responded with unexpected HTTP status {status}")]
    UnexpectedResponse {
        /// The url that was being fetched.
        url: String,
        /// The HTTP status code that came back.
        status: u16,
    },

    /// A meta API response could not be deserialized.
    #[error("failed to parse the response from {url}: {source}")]
    Json {
        /// The url that was being fetched.
        url: String,
        /// The underlying deserialization failure.
        #[source]
        source: serde_json::Error,
    },

    /// A file download failed.
    #[error("failed to download {url}: {source}")]
    Download {
        /// The url that was being downloaded.
        url: String,
        /// The underlying failure.
        #[source]
        source: Box<dyn std::error::Error + Send + Sync>,
    },

    /// A download target directory could not be created.
    #[error("failed to create {}", path.display())]
    Io {
        /// The path that could not be created.
        path: PathBuf,
        /// The underlying failure.
        #[source]
        source: std::io::Error,
    },

    /// `download_client` was called with a version that is not bound to a
    /// Minecraft version (it came from `list_versions`).
    #[error("loader version {version} is not bound to a Minecraft version; fetch it with versions_for_minecraft instead of list_versions")]
    MinecraftVersionUnknown {
        /// The version that has no Minecraft binding.
        version: String,
    },

    /// The requested loader version does not exist for the requested Minecraft version.
    #[error("loader version {version} is not available for Minecraft {minecraft}")]
    VersionNotFound {
        /// The loader version that was requested.
        version: String,
        /// The Minecraft version it was requested for.
        minecraft: String,
    },

    /// `extract_mod_info` was called on a kind whose jars carry no mod metadata.
    #[error("the {loader} loader does not support mod metadata")]
    ModInfoUnsupported {
        /// The loader that was asked for mod metadata.
        loader: &'static str,
    },

    /// The jar carries no client-mod metadata this crate supports (this includes
    /// server-plugin jars, which are out of scope).
    #[error("no mod metadata found in {}", path.display())]
    ModMetadataMissing {
        /// The jar that was inspected.
        path: PathBuf,
    },

    /// A metadata file was found but could not be parsed.
    #[error("failed to parse mod metadata in {}: {source}", path.display())]
    ModParse {
        /// The jar that was inspected.
        path: PathBuf,
        /// The underlying parse failure.
        #[source]
        source: Box<dyn std::error::Error + Send + Sync>,
    },

    /// The loader has no install step of its own. Vanilla installs through piston-mc.
    #[error("the {loader} loader has no install step of its own")]
    InstallUnsupported {
        /// The loader that was asked to install.
        loader: &'static str,
    },

    /// The Java program that runs a loader installer could not be started.
    #[error("could not start {}: {source}", program.display())]
    InstallerLaunch {
        /// The Java program meant to run the installer.
        program: PathBuf,
        /// The underlying failure.
        #[source]
        source: std::io::Error,
    },

    /// A loader installer ran and did not succeed. `output` holds the last lines it printed.
    #[error("the {loader} installer failed ({status}):\n{output}")]
    InstallerFailed {
        /// The loader whose installer failed.
        loader: &'static str,
        /// The exit code, or a note that the installer was stopped.
        status: String,
        /// The last lines the installer printed.
        output: String,
    },

    /// A loader installer exited cleanly but left no new version folder behind.
    #[error("the {loader} installer did not create a version in {}", directory.display())]
    InstallerVersionMissing {
        /// The loader whose installer finished.
        loader: &'static str,
        /// The game folder the installer wrote into.
        directory: PathBuf,
    },

    /// A loader's install did not download the installer jar it runs.
    #[error("the {loader} installer was not downloaded")]
    InstallerMissing {
        /// The loader whose installer is missing.
        loader: &'static str,
    },
}
