//! The launchable Minecraft instance — the Library's domain model.

use chrono::{DateTime, Local};

/// The mod loader an instance runs through.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum Loader {
    Vanilla,
    Fabric,
    Forge,
    Quilt,
    NeoForge,
}

impl Loader {
    /// The uppercase label the loader chip displays.
    pub fn label(self) -> &'static str {
        match self {
            Loader::Vanilla => "VANILLA",
            Loader::Fabric => "FABRIC",
            Loader::Forge => "FORGE",
            Loader::Quilt => "QUILT",
            Loader::NeoForge => "NEOFORGE",
        }
    }

    pub(crate) fn from_db(value: &str) -> Self {
        match value {
            "fabric" => Loader::Fabric,
            "forge" => Loader::Forge,
            "quilt" => Loader::Quilt,
            "neoforge" => Loader::NeoForge,
            _ => Loader::Vanilla,
        }
    }

    /// Only the tests write rows until the New Instance flow exists.
    #[cfg(test)]
    pub(crate) fn to_db(self) -> &'static str {
        match self {
            Loader::Vanilla => "vanilla",
            Loader::Fabric => "fabric",
            Loader::Forge => "forge",
            Loader::Quilt => "quilt",
            Loader::NeoForge => "neoforge",
        }
    }
}

/// Whether the instance is currently running.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum InstanceState {
    Idle,
    Playing,
}

impl InstanceState {
    pub(crate) fn from_db(value: &str) -> Self {
        match value {
            "playing" => InstanceState::Playing,
            _ => InstanceState::Idle,
        }
    }

    /// Only the tests write rows until the New Instance flow exists.
    #[cfg(test)]
    pub(crate) fn to_db(self) -> &'static str {
        match self {
            InstanceState::Idle => "idle",
            InstanceState::Playing => "playing",
        }
    }
}

/// The biome palette the generated scene art uses.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum Biome {
    End,
    Forest,
    Cherry,
    Ocean,
    Desert,
    Snow,
    Nether,
    Mushroom,
}

impl Biome {
    pub(crate) fn from_db(value: &str) -> Self {
        match value {
            "cherry" => Biome::Cherry,
            "ocean" => Biome::Ocean,
            "desert" => Biome::Desert,
            "snow" => Biome::Snow,
            "nether" => Biome::Nether,
            "mushroom" => Biome::Mushroom,
            "forest" => Biome::Forest,
            _ => Biome::End,
        }
    }

    /// Only the tests write rows until the New Instance flow exists.
    #[cfg(test)]
    pub(crate) fn to_db(self) -> &'static str {
        match self {
            Biome::End => "end",
            Biome::Forest => "forest",
            Biome::Cherry => "cherry",
            Biome::Ocean => "ocean",
            Biome::Desert => "desert",
            Biome::Snow => "snow",
            Biome::Nether => "nether",
            Biome::Mushroom => "mushroom",
        }
    }
}

/// The launchable Minecraft instance.
#[derive(Debug, Clone)]
pub struct Instance {
    /// Stable identifier (uuid) — also the primary key in the database.
    pub id: String,
    /// Display name ("Aether Legacy").
    pub name: String,
    /// The Minecraft version the instance targets ("1.20.1").
    pub mc_version: String,
    pub loader: Loader,
    /// The loader's own version ("0.14.21"), when modded.
    pub loader_version: Option<String>,
    /// Total playtime in seconds.
    pub playtime_seconds: u32,
    pub mod_count: u32,
    /// When the instance was last played; `None` = installed, never played.
    pub last_played: Option<DateTime<Local>>,
    pub installed_at: DateTime<Local>,
    pub pinned: bool,
    pub state: InstanceState,
    /// The biome palette the generated scene art uses.
    pub biome: Biome,
    /// The terrain-silhouette variation for the generated scene art.
    pub art_seed: u64,
    /// The instance's folder name inside the instances directory.
    pub directory_name: String,
}

impl Instance {
    /// The sort key for "most recent": last played, else installed.
    pub fn recency(&self) -> DateTime<Local> {
        self.last_played.unwrap_or(self.installed_at)
    }
}

/// A minimal instance for the tests: installed now, never played, no mods.
#[cfg(test)]
pub(crate) fn fixture(id: &str, name: &str, loader: Loader) -> Instance {
    Instance {
        id: id.to_string(),
        name: name.to_string(),
        mc_version: "1.20.1".to_string(),
        loader,
        loader_version: None,
        playtime_seconds: 0,
        mod_count: 0,
        last_played: None,
        installed_at: Local::now(),
        pinned: false,
        state: InstanceState::Idle,
        biome: Biome::Forest,
        art_seed: 1,
        directory_name: id.to_string(),
    }
}

#[cfg(test)]
#[path = "../../../../tests/app/pages/library/instance.rs"]
mod tests;
