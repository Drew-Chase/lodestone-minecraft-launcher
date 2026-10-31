//! The launchable Minecraft instance — the Library's domain model.

use std::path::{Path, PathBuf};

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

/// The instance's square icon: a background preset with a symbol, or an
/// uploaded image.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum InstanceIcon {
    /// Composed from the icon catalog, both parts by id. An unknown id falls
    /// back to the catalog's first entry when it is drawn.
    Composed { background: String, symbol: String },
    /// An uploaded square image. Stored relative to the instance folder;
    /// [`Instance::resolve_files`] makes it absolute after a load.
    Custom(PathBuf),
}

impl InstanceIcon {
    /// The icon's `(background, symbol, custom path)` columns.
    pub(crate) fn to_db(&self) -> (Option<&str>, Option<&str>, Option<String>) {
        match self {
            InstanceIcon::Composed { background, symbol } => (Some(background.as_str()), Some(symbol.as_str()), None),
            InstanceIcon::Custom(path) => (None, None, Some(path.to_string_lossy().into_owned())),
        }
    }

    /// Reads the icon columns back. A row without a custom path is composed,
    /// and a missing background or symbol is left empty for the catalog to
    /// fill in.
    pub(crate) fn from_db(background: Option<String>, symbol: Option<String>, custom: Option<String>) -> Self {
        match custom {
            Some(path) => InstanceIcon::Custom(PathBuf::from(path)),
            None => InstanceIcon::Composed {
                background: background.unwrap_or_default(),
                symbol: symbol.unwrap_or_default(),
            },
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
    /// The uploaded banner. Stored relative to the instance folder;
    /// [`Instance::resolve_files`] makes it absolute after a load. `None`
    /// shows the scene art.
    pub art_path: Option<PathBuf>,
    /// The square icon shown on cards and the instance page.
    pub icon: InstanceIcon,
    /// The instance's folder name inside the instances directory.
    pub directory_name: String,
    /// The bundled Java runtime's short name ("delta"), once installed.
    pub java_component: Option<String>,
    /// The version the launcher starts ("fabric-loader-0.16.10-1.21.4"), once installed.
    pub version_id: Option<String>,
    /// The page the instance was installed from, when it came from an external modpack source.
    /// Locally created instances have none.
    pub source_url: Option<String>,
}

impl Instance {
    /// The sort key for "most recent": last played, else installed.
    pub fn recency(&self) -> DateTime<Local> {
        self.last_played.unwrap_or(self.installed_at)
    }

    /// Makes the stored file paths absolute, against the instances directory.
    pub fn resolve_files(&mut self, instances_dir: &Path) {
        let folder = instances_dir.join(&self.directory_name);
        if let Some(path) = &mut self.art_path {
            *path = folder.join(&*path);
        }
        if let InstanceIcon::Custom(path) = &mut self.icon {
            *path = folder.join(&*path);
        }
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
        art_path: None,
        icon: InstanceIcon::Composed {
            background: "rose".to_string(),
            symbol: "grass".to_string(),
        },
        directory_name: id.to_string(),
        java_component: None,
        version_id: None,
        source_url: None,
    }
}

#[cfg(test)]
#[path = "../../../../tests/app/pages/library/instance.rs"]
mod tests;
