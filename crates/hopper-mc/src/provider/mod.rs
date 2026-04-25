//! Provider traits — implemented once per platform.

pub mod traits;

pub use traits::{
    ContentProvider, DatapackProvider, ModProvider, PackProvider, ResourcePackProvider,
    ShaderPackProvider, VersionProvider, WorldProvider,
};
