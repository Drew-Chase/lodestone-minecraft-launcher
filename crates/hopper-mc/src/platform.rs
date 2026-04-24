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
