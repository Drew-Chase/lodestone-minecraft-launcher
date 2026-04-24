use serde::{Deserialize, Serialize};

use super::common::ContentBase;

/// A Minecraft datapack.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DatapackItem {
    #[serde(flatten)]
    pub base: ContentBase,

    /// Datapack `pack_format` version, if the platform exposes it.
    pub pack_format: Option<u32>,
}
