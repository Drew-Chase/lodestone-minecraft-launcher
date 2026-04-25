//! Shared model types used across every content kind.

use std::collections::HashMap;

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};

use crate::platform::Platform;

/// Fields every content item carries regardless of its kind.
///
/// Item structs (`ModItem`, `PackItem`, ...) flatten this via
/// `#[serde(flatten)]` so call sites see e.g. `item.title` directly
/// rather than having to go through `item.base.title`.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ContentBase {
    /// Platform-assigned identifier (opaque string, platform-specific shape).
    pub id: String,

    /// URL-friendly slug (may be equal to `id` on platforms that don't split the two).
    pub slug: String,

    /// Which platform this item came from.
    pub platform: Platform,

    /// Display title.
    pub title: String,

    /// Short description shown in lists.
    pub summary: String,

    /// Full long-form description. Typically only populated by `get_*` calls;
    /// discovery/search calls leave this `None`.
    pub description: Option<String>,

    pub authors: Vec<Author>,

    pub icon_url: Option<String>,

    /// Screenshot / preview image URLs.
    pub gallery: Vec<String>,

    pub links: Links,

    pub downloads: u64,

    pub follows: u64,

    pub license: Option<License>,

    pub created: DateTime<Utc>,

    pub updated: DateTime<Utc>,

    /// Free-form category tags as reported by the platform.
    pub categories: Vec<String>,

    /// Minecraft versions supported by this item.
    pub game_versions: Vec<String>,
}

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct Author {
    pub name: String,
    pub url: Option<String>,
}

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct Links {
    pub website: Option<String>,
    pub source: Option<String>,
    pub issues: Option<String>,
    pub wiki: Option<String>,
    pub discord: Option<String>,
    pub donation: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct License {
    /// SPDX identifier or platform-specific license id.
    pub id: String,
    pub name: Option<String>,
    pub url: Option<String>,
}

/// A dependency relationship from one piece of content to another.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Dependency {
    /// Id of the dependency on the same platform, if known.
    pub project_id: Option<String>,
    /// Pinned file/version id of the dependency, if known.
    pub version_id: Option<String>,
    pub kind: DependencyKind,
}

#[derive(Debug, Copy, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum DependencyKind {
    Required,
    Optional,
    Incompatible,
    Embedded,
}

/// A released version of a project, containing one or more downloadable files.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ProjectVersion {
    pub id: String,
    pub project_id: String,
    /// Display name (e.g. "Sodium 0.6.0 for Minecraft 1.21.4").
    pub name: String,
    /// Semantic version string (e.g. "0.6.0").
    pub version_number: String,
    /// Markdown or HTML changelog, if provided.
    pub changelog: Option<String>,
    pub date_published: DateTime<Utc>,
    pub downloads: u64,
    pub version_type: VersionType,
    /// Minecraft versions this release supports.
    pub game_versions: Vec<String>,
    /// Loader names (e.g. `["fabric", "quilt"]`).
    pub loaders: Vec<String>,
    pub files: Vec<VersionFile>,
    pub dependencies: Vec<Dependency>,
    pub featured: bool,
    pub platform: Platform,
}

/// Release channel of a version.
#[derive(Debug, Copy, Clone, PartialEq, Eq, Default, Serialize, Deserialize)]
pub enum VersionType {
    #[default]
    Release,
    Beta,
    Alpha,
}

/// A downloadable file within a [`ProjectVersion`].
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct VersionFile {
    /// Direct download URL. `None` when the platform restricts
    /// third-party distribution (e.g. some CurseForge projects).
    pub url: Option<String>,
    pub filename: String,
    /// Size in bytes.
    pub size: u64,
    /// Whether this is the primary file for the version.
    pub primary: bool,
    /// Hash algorithm → hex digest (e.g. `{"sha1": "abc…", "sha512": "def…"}`).
    pub hashes: HashMap<String, String>,
}

/// How a mod interacts with the client/server side.
#[derive(Debug, Default, Copy, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum SideSupport {
    Required,
    Optional,
    Unsupported,
    #[default]
    Unknown,
}
