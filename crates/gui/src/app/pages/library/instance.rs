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

    /// Builds the seed instances shown before a real instance manager
    /// exists. Deterministic, varied across loaders, biomes and playtime.
    pub fn seed_pack(now: DateTime<Local>) -> Vec<Self> {
        const NAMES: [&str; 16] = [
            "Aether Legacy",
            "Vanilla Survival",
            "Create: Above & Beyond",
            "Sky Factory 5",
            "Better MC Forever",
            "DawnCraft",
            "Prominence II",
            "Medieval Madness",
            "Terralith Exploration",
            "All the Mods 10",
            "Retro NES",
            "Cave & Cliffs Reborn",
            "Pixelmon Adventures",
            "Vault Hunters 3",
            "Blue Skies",
            "Hardcore World",
        ];
        const LOADERS: [Loader; 5] = [Loader::Fabric, Loader::Vanilla, Loader::Forge, Loader::Quilt, Loader::NeoForge];
        const BIOME_CYCLE: [Biome; 8] = [
            Biome::End,
            Biome::Forest,
            Biome::Cherry,
            Biome::Ocean,
            Biome::Desert,
            Biome::Snow,
            Biome::Nether,
            Biome::Mushroom,
        ];
        const LOADER_VERSIONS: [&str; 5] = ["0.14.21", "", "1.16.5", "0.15.11", "20.4.237"];

        (0..48)
            .map(|index| {
                let name = NAMES[index % NAMES.len()].to_string();
                let loader = LOADERS[index % LOADERS.len()];
                let played = index % 5 != 4; // every fifth never played
                let hours = (index * 37) % 300;
                let last_played = played.then(|| now - chrono::Duration::hours((index * 13 + 12) as i64));
                let installed_at = now - chrono::Duration::days((index * 5 + 3) as i64);
                let slug = name.to_lowercase().replace([':', ' ', '&'], "-");
                // The reference instance (04-instance.png) is Aether
                // Legacy: 87 mods, 24h 12m played, a suffix-free folder.
                let playtime_seconds = if index == 0 { 24 * 3600 + 12 * 60 } else { hours * 3600 };
                Instance {
                    id: format!("seed-{index:03}"),
                    name,
                    mc_version: match index % 4 {
                        0 => "1.20.1".into(),
                        1 => "1.20.4".into(),
                        2 => "1.16.5".into(),
                        _ => "1.19.4".into(),
                    },
                    loader,
                    loader_version: if loader == Loader::Vanilla {
                        None
                    } else {
                        Some(LOADER_VERSIONS[index % LOADER_VERSIONS.len()].into())
                    },
                    playtime_seconds: playtime_seconds as u32,
                    mod_count: match loader {
                        Loader::Vanilla => 0,
                        _ if index == 0 => 87,
                        _ => ((index * 23 + 37) % 240) as u32,
                    },
                    last_played,
                    installed_at,
                    pinned: index % 7 == 3,
                    state: if index == 0 { InstanceState::Playing } else { InstanceState::Idle },
                    biome: BIOME_CYCLE[index % BIOME_CYCLE.len()],
                    art_seed: (index as u64) * 7 + 1,
                    directory_name: if index == 0 { slug } else { format!("{slug}-{:04}", index + 1) },
                }
            })
            .collect()
    }
}

#[cfg(test)]
#[path = "../../../../tests/app/pages/library/instance.rs"]
mod tests;
