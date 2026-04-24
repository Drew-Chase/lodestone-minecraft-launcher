use serde::{Deserialize, Serialize};

use super::common::ContentBase;

/// A downloadable Minecraft world / save.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WorldItem {
    #[serde(flatten)]
    pub base: ContentBase,

    /// Minecraft version the world was saved in, if reported.
    pub mc_version: Option<String>,

    /// Size of the world download in bytes, if known.
    pub size_bytes: Option<u64>,
}
