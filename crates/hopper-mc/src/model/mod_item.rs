use serde::{Deserialize, Serialize};

use super::common::{ContentBase, Dependency, SideSupport};

/// A Minecraft mod.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ModItem {
    #[serde(flatten)]
    pub base: ContentBase,

    /// Mod loaders this mod is compatible with (e.g. "fabric", "forge", "quilt", "neoforge").
    pub loaders: Vec<String>,

    pub client_side: SideSupport,
    pub server_side: SideSupport,

    /// Declared dependencies on other mods. Only populated by `get_mod`.
    pub dependencies: Vec<Dependency>,
}
