use serde::{Deserialize, Serialize};

use super::common::ContentBase;

/// A Minecraft modpack.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PackItem {
    #[serde(flatten)]
    pub base: ContentBase,

    /// Mod loaders bundled in the pack.
    pub loaders: Vec<String>,

    /// Primary Minecraft version the pack targets, if the platform advertises it.
    pub mc_version: Option<String>,

    /// Number of mods bundled, if known.
    pub included_mods_count: Option<u32>,

    /// Whether a server-side variant of the pack is distributed.
    pub has_server_pack: bool,
}
