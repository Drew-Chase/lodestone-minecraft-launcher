use serde::{Deserialize, Serialize};

use super::common::ContentBase;

/// A Minecraft shader pack.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ShaderPackItem {
    #[serde(flatten)]
    pub base: ContentBase,

    /// Shader loaders this pack supports (e.g. "iris", "optifine", "canvas", "vanilla").
    pub shader_loaders: Vec<String>,
}
