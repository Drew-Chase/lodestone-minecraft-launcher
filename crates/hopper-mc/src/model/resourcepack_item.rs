use serde::{Deserialize, Serialize};

use super::common::ContentBase;

/// A Minecraft resource pack.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ResourcePackItem {
    #[serde(flatten)]
    pub base: ContentBase,

    /// Resource pack `pack_format`, if the platform exposes it.
    pub pack_format: Option<u32>,

    /// Pixel resolution of textures (e.g. 16, 32, 64, ...), if advertised.
    pub resolution: Option<u32>,
}
