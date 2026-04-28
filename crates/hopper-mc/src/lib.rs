//! # hopper-mc
//!
//! Like its namesake block, `hopper-mc` funnels Minecraft content from
//! many sources into one output. It is a unified async client that
//! aggregates discovery of mods, modpacks, datapacks, resource packs,
//! worlds, and shader packs across Modrinth, CurseForge, AT Launcher,
//! TechnicPack, and FTB — exposing them through a single common API.
//!
//! ## High-level API
//!
//! The top-level [`find_mods`], [`find_packs`], [`get_mod`], ... functions
//! dispatch to the appropriate platform provider based on a [`Platform`]
//! argument. Platforms that cannot serve a given content type return
//! [`ContentError::UnsupportedContentType`]; skeleton platforms that have
//! yet to be implemented return [`ContentError::NotImplemented`].
//!
//! ```no_run
//! use hopper_mc::{find_mods, find_packs, get_mod, Platform, SearchFilters, Sort};
//!
//! # async fn example() -> Result<(), hopper_mc::ContentError> {
//! let page = 0u32;
//! let per = 10u32;
//! let filters = SearchFilters::default();
//!
//! let modrinth_mods      = find_mods(None, Sort::Downloads, &filters, Platform::Modrinth, page, per).await?;
//! let search_modrinth    = find_mods(Some("fabric api"), Sort::Latest, &filters, Platform::Modrinth, page, per).await?;
//! let one_mod            = get_mod("fabric-api", Platform::Modrinth).await?;
//! let modrinth_modpacks  = find_packs(None, Sort::Latest, &filters, Platform::Modrinth, page, per).await?;
//! # let _ = (modrinth_mods, search_modrinth, one_mod, modrinth_modpacks);
//! # Ok(()) }
//! ```
//!
//! ## Expansibility
//!
//! Each content kind has its own provider trait
//! ([`ModProvider`](provider::ModProvider),
//! [`PackProvider`](provider::PackProvider), ...). Platforms implement the
//! subset they can serve. Adding a new platform is:
//!
//! 1. Add a variant to [`Platform`].
//! 2. Create a new module under `platforms::` with a struct that implements
//!    [`ContentProvider`](provider::ContentProvider) plus the per-kind traits
//!    it supports.
//! 3. Add `match` arms to the top-level dispatch functions in this file.

pub mod error;
pub mod model;
pub mod modpack;
pub mod platform;
pub mod platforms;
pub mod provider;

pub use error::{ContentError, Result};
pub use model::{
    Author, ContentBase, DatapackItem, Dependency, DependencyKind, License, Links, ModItem,
    PackItem, ProjectVersion, ResourcePackItem, ShaderPackItem, SideSupport, VersionFile,
    VersionType, WorldItem,
};
pub use modpack::{
    extract_overrides, parse_curseforge_pack, parse_modpack, parse_mrpack, ModpackFile,
    ModpackFileEnv, ModpackManifest, ModpackSource,
};
pub use platform::{ContentType, Platform, SearchFilters, Sort};
pub use platforms::{
    AtLauncherProvider, CurseForgeProvider, FtbProvider, ModrinthProvider, TechnicProvider,
};
pub use provider::{
    ContentProvider, DatapackProvider, ModProvider, PackProvider, ResourcePackProvider,
    ShaderPackProvider, VersionProvider, WorldProvider,
};

// ---------------------------------------------------------------------------
// Top-level dispatch
// ---------------------------------------------------------------------------

macro_rules! unsupported {
    ($platform:expr, $kind:expr) => {
        Err(ContentError::UnsupportedContentType {
            platform: $platform,
            kind: $kind,
        })
    };
}

/// Search for mods on the given platform.
pub async fn find_mods(
    query: Option<&str>,
    sort: Sort,
    filters: &SearchFilters,
    platform: Platform,
    page: u32,
    per_page: u32,
) -> Result<Vec<ModItem>> {
    match platform {
        Platform::Modrinth => {
            ModrinthProvider::shared()
                .find_mods(query, sort, filters, page, per_page)
                .await
        }
        Platform::CurseForge => {
            CurseForgeProvider::shared()
                .find_mods(query, sort, filters, page, per_page)
                .await
        }
        Platform::AtLauncher | Platform::Technic | Platform::Ftb => {
            unsupported!(platform, ContentType::Mod)
        }
    }
}

/// Look up a single mod by its platform-specific id or slug.
pub async fn get_mod(id: &str, platform: Platform) -> Result<Option<ModItem>> {
    match platform {
        Platform::Modrinth => ModrinthProvider::shared().get_mod(id).await,
        Platform::CurseForge => CurseForgeProvider::shared().get_mod(id).await,
        Platform::AtLauncher | Platform::Technic | Platform::Ftb => {
            unsupported!(platform, ContentType::Mod)
        }
    }
}

/// Search for modpacks on the given platform.
pub async fn find_packs(
    query: Option<&str>,
    sort: Sort,
    filters: &SearchFilters,
    platform: Platform,
    page: u32,
    per_page: u32,
) -> Result<Vec<PackItem>> {
    match platform {
        Platform::Modrinth => {
            ModrinthProvider::shared()
                .find_packs(query, sort, filters, page, per_page)
                .await
        }
        Platform::CurseForge => {
            CurseForgeProvider::shared()
                .find_packs(query, sort, filters, page, per_page)
                .await
        }
        Platform::AtLauncher => {
            AtLauncherProvider::shared()
                .find_packs(query, sort, filters, page, per_page)
                .await
        }
        Platform::Technic => {
            TechnicProvider::shared()
                .find_packs(query, sort, filters, page, per_page)
                .await
        }
        Platform::Ftb => {
            FtbProvider::shared()
                .find_packs(query, sort, filters, page, per_page)
                .await
        }
    }
}

/// Look up a single modpack by its platform-specific id or slug.
pub async fn get_pack(id: &str, platform: Platform) -> Result<Option<PackItem>> {
    match platform {
        Platform::Modrinth => ModrinthProvider::shared().get_pack(id).await,
        Platform::CurseForge => CurseForgeProvider::shared().get_pack(id).await,
        Platform::AtLauncher => AtLauncherProvider::shared().get_pack(id).await,
        Platform::Technic => TechnicProvider::shared().get_pack(id).await,
        Platform::Ftb => FtbProvider::shared().get_pack(id).await,
    }
}

/// Search for datapacks on the given platform.
pub async fn find_datapacks(
    query: Option<&str>,
    sort: Sort,
    filters: &SearchFilters,
    platform: Platform,
    page: u32,
    per_page: u32,
) -> Result<Vec<DatapackItem>> {
    match platform {
        Platform::Modrinth => {
            ModrinthProvider::shared()
                .find_datapacks(query, sort, filters, page, per_page)
                .await
        }
        Platform::CurseForge => {
            CurseForgeProvider::shared()
                .find_datapacks(query, sort, filters, page, per_page)
                .await
        }
        Platform::AtLauncher | Platform::Technic | Platform::Ftb => {
            unsupported!(platform, ContentType::Datapack)
        }
    }
}

/// Look up a single datapack by its platform-specific id or slug.
pub async fn get_datapack(id: &str, platform: Platform) -> Result<Option<DatapackItem>> {
    match platform {
        Platform::Modrinth => ModrinthProvider::shared().get_datapack(id).await,
        Platform::CurseForge => CurseForgeProvider::shared().get_datapack(id).await,
        Platform::AtLauncher | Platform::Technic | Platform::Ftb => {
            unsupported!(platform, ContentType::Datapack)
        }
    }
}

/// Search for resource packs on the given platform.
pub async fn find_resourcepacks(
    query: Option<&str>,
    sort: Sort,
    filters: &SearchFilters,
    platform: Platform,
    page: u32,
    per_page: u32,
) -> Result<Vec<ResourcePackItem>> {
    match platform {
        Platform::Modrinth => {
            ModrinthProvider::shared()
                .find_resourcepacks(query, sort, filters, page, per_page)
                .await
        }
        Platform::CurseForge => {
            CurseForgeProvider::shared()
                .find_resourcepacks(query, sort, filters, page, per_page)
                .await
        }
        Platform::AtLauncher | Platform::Technic | Platform::Ftb => {
            unsupported!(platform, ContentType::ResourcePack)
        }
    }
}

/// Look up a single resource pack by its platform-specific id or slug.
pub async fn get_resourcepack(
    id: &str,
    platform: Platform,
) -> Result<Option<ResourcePackItem>> {
    match platform {
        Platform::Modrinth => ModrinthProvider::shared().get_resourcepack(id).await,
        Platform::CurseForge => CurseForgeProvider::shared().get_resourcepack(id).await,
        Platform::AtLauncher | Platform::Technic | Platform::Ftb => {
            unsupported!(platform, ContentType::ResourcePack)
        }
    }
}

/// Search for shader packs on the given platform.
pub async fn find_shaderpacks(
    query: Option<&str>,
    sort: Sort,
    filters: &SearchFilters,
    platform: Platform,
    page: u32,
    per_page: u32,
) -> Result<Vec<ShaderPackItem>> {
    match platform {
        Platform::Modrinth => {
            ModrinthProvider::shared()
                .find_shaderpacks(query, sort, filters, page, per_page)
                .await
        }
        Platform::CurseForge => {
            CurseForgeProvider::shared()
                .find_shaderpacks(query, sort, filters, page, per_page)
                .await
        }
        Platform::AtLauncher | Platform::Technic | Platform::Ftb => {
            unsupported!(platform, ContentType::ShaderPack)
        }
    }
}

/// Look up a single shader pack by its platform-specific id or slug.
pub async fn get_shaderpack(id: &str, platform: Platform) -> Result<Option<ShaderPackItem>> {
    match platform {
        Platform::Modrinth => ModrinthProvider::shared().get_shaderpack(id).await,
        Platform::CurseForge => CurseForgeProvider::shared().get_shaderpack(id).await,
        Platform::AtLauncher | Platform::Technic | Platform::Ftb => {
            unsupported!(platform, ContentType::ShaderPack)
        }
    }
}

/// Search for downloadable worlds on the given platform.
pub async fn find_worlds(
    query: Option<&str>,
    sort: Sort,
    filters: &SearchFilters,
    platform: Platform,
    page: u32,
    per_page: u32,
) -> Result<Vec<WorldItem>> {
    match platform {
        Platform::Modrinth => {
            ModrinthProvider::shared()
                .find_worlds(query, sort, filters, page, per_page)
                .await
        }
        Platform::CurseForge => {
            CurseForgeProvider::shared()
                .find_worlds(query, sort, filters, page, per_page)
                .await
        }
        Platform::AtLauncher | Platform::Technic | Platform::Ftb => {
            unsupported!(platform, ContentType::World)
        }
    }
}

/// Look up a single world by its platform-specific id or slug.
pub async fn get_world(id: &str, platform: Platform) -> Result<Option<WorldItem>> {
    match platform {
        Platform::Modrinth => ModrinthProvider::shared().get_world(id).await,
        Platform::CurseForge => CurseForgeProvider::shared().get_world(id).await,
        Platform::AtLauncher | Platform::Technic | Platform::Ftb => {
            unsupported!(platform, ContentType::World)
        }
    }
}

/// List all versions (with files and dependencies) for a project.
pub async fn get_versions(project_id: &str, platform: Platform) -> Result<Vec<ProjectVersion>> {
    match platform {
        Platform::Modrinth => ModrinthProvider::shared().get_versions(project_id).await,
        Platform::CurseForge => CurseForgeProvider::shared().get_versions(project_id).await,
        Platform::AtLauncher | Platform::Technic | Platform::Ftb => {
            Err(ContentError::NotImplemented(platform))
        }
    }
}

/// Fetch a single version by its platform-specific id.
pub async fn get_version(version_id: &str, platform: Platform) -> Result<Option<ProjectVersion>> {
    match platform {
        Platform::Modrinth => ModrinthProvider::shared().get_version(version_id).await,
        Platform::CurseForge => CurseForgeProvider::shared().get_version(version_id).await,
        Platform::AtLauncher | Platform::Technic | Platform::Ftb => {
            Err(ContentError::NotImplemented(platform))
        }
    }
}
