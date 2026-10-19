//! Errors surfaced by browsing, downloading, and parsing platform content.

use std::path::PathBuf;

use crate::meta::ContentType;

/// Everything that can go wrong in this crate. Messages are written to be shown
/// to a user verbatim: gui surfaces them without extra formatting, and each
/// variant carries the context needed to explain itself.
#[derive(Debug, thiserror::Error)]
pub enum PlatformError {
    /// A platform API could not be reached at all.
    #[error("failed to reach {url}: {source}")]
    Network {
        /// The url that was being fetched.
        url: String,
        /// The underlying transport failure.
        #[source]
        source: reqwest::Error,
    },

    /// A platform API answered, but with an unexpected HTTP status.
    #[error("{url} responded with unexpected HTTP status {status}")]
    UnexpectedResponse {
        /// The url that was being fetched.
        url: String,
        /// The HTTP status code that came back.
        status: u16,
    },

    /// A platform API response could not be deserialized.
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

    /// The requested content type is outside the platform's capability matrix.
    #[error("{platform} does not host {content} content")]
    UnsupportedContentType {
        /// The platform that was asked.
        platform: &'static str,
        /// The content type that was requested.
        content: ContentType,
    },

    /// The requested action is not offered by the platform at all.
    #[error("{platform} does not support {action}")]
    UnsupportedPlatform {
        /// The platform that was asked.
        platform: &'static str,
        /// The action that was requested, e.g. "modpack archive parsing".
        action: &'static str,
    },

    /// The archive contains no known modpack manifest.
    #[error("{} is not a recognized modpack archive", path.display())]
    ModpackFormatUnknown {
        /// The archive that was inspected.
        path: PathBuf,
    },

    /// A modpack manifest was found but could not be parsed.
    #[error("failed to parse the modpack manifest in {}: {source}", path.display())]
    ModpackParse {
        /// The archive that was inspected.
        path: PathBuf,
        /// The underlying parse failure.
        #[source]
        source: Box<dyn std::error::Error + Send + Sync>,
    },
}
