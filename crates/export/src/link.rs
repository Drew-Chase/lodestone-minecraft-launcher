//! The links a package records for a file, and the entries the writers take. A file with a link is listed
//! by its origin; a file without one is bundled into the package.

use std::path::PathBuf;

use serde::{Deserialize, Serialize};

use crate::hash::Digests;

/// Where a file is on Modrinth: its project, its release, the project's page, and the file's download.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ModrinthLink {
    /// The project's id, such as `AANobbMI`.
    pub project: String,
    /// The release's id.
    pub version: String,
    /// The project's page. Modrinth serves it for every project type.
    pub page: String,
    /// The file's direct download url.
    pub download_url: String,
}

/// Where a file is on CurseForge: its project and its file, by their numeric ids, as text.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct CurseForgeLink {
    pub project: String,
    pub file: String,
}

/// The links a file has on each platform.
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Links {
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub modrinth: Option<ModrinthLink>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub curseforge: Option<CurseForgeLink>,
}

impl Links {
    /// Whether the file has no link on either platform.
    pub fn is_empty(&self) -> bool {
        self.modrinth.is_none() && self.curseforge.is_none()
    }
}

/// One file a package takes.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Entry {
    /// The path inside the instance, with `/` separators.
    pub path: String,
    /// Where the bytes are on disk.
    pub source: PathBuf,
    /// The file's size in bytes.
    pub size: u64,
    /// The file's digests. Set for the mods and resource files a package may link; `None` for the rest.
    pub digests: Option<Digests>,
    /// The file's links.
    pub links: Links,
}

impl Entry {
    /// Whether the file has a link on either platform.
    pub fn is_linked(&self) -> bool {
        !self.links.is_empty()
    }
}
