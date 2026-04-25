//! Platform, Sort, and ContentType enums used throughout the crate.

use serde::{Deserialize, Serialize};

/// A content distribution platform.
#[derive(Debug, Copy, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum Platform {
    Modrinth,
    CurseForge,
    AtLauncher,
    Technic,
    Ftb,
}

impl Platform {
    /// A human-readable name for the platform.
    pub fn display_name(self) -> &'static str {
        match self {
            Platform::Modrinth => "Modrinth",
            Platform::CurseForge => "CurseForge",
            Platform::AtLauncher => "AT Launcher",
            Platform::Technic => "TechnicPack",
            Platform::Ftb => "Feed The Beast",
        }
    }
}

/// Sort order for discovery queries.
///
/// Each platform maps these to its own sort keys; see the platform module
/// for details. Platforms that cannot support a given sort will silently
/// fall back to the closest equivalent.
#[derive(Debug, Copy, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum Sort {
    /// Best match for the query string. Equivalent to `Latest` when no query is given.
    Relevance,
    /// Most downloads first.
    Downloads,
    /// Most followers/likes first.
    Follows,
    /// Most recently created first.
    Latest,
    /// Most recently updated first.
    Updated,
}

/// The kind of content being queried. Used by dispatch and in errors
/// when a platform cannot serve a given type.
#[derive(Debug, Copy, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum ContentType {
    Mod,
    Modpack,
    Datapack,
    ResourcePack,
    ShaderPack,
    World,
}

/// Optional filters applied to search/find queries.
///
/// All fields default to empty/`None`, meaning no filtering. Platforms map
/// these to their own query parameters or facets as best they can — fields
/// that a platform cannot represent are silently ignored.
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct SearchFilters {
    /// Category tags to require (e.g. `["adventure", "tech"]`).
    pub categories: Vec<String>,
    /// Loader names to require (e.g. `["fabric", "forge"]`).
    pub loaders: Vec<String>,
    /// Minecraft version strings to require (e.g. `["1.20.1", "1.21.4"]`).
    pub versions: Vec<String>,
    /// Client-side support filter (`"required"`, `"optional"`, `"unsupported"`).
    pub client_side: Option<String>,
    /// Server-side support filter (`"required"`, `"optional"`, `"unsupported"`).
    pub server_side: Option<String>,
}
