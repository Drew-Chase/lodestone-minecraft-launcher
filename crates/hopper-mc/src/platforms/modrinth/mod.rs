//! Modrinth provider — full implementation.
//!
//! Uses the Labrinth v2 HTTP API at `https://api.modrinth.com/v2`. Modrinth
//! asks that clients identify themselves with a descriptive `User-Agent`
//! (`<name>/<version>`) — we set one at construction time.

mod api;
mod dto;
mod mapping;

use std::sync::OnceLock;

use crate::error::{ContentError, Result};
use crate::model::{
    DatapackItem, ModItem, PackItem, ResourcePackItem, ShaderPackItem, WorldItem,
};
use crate::platform::{ContentType, Platform, SearchFilters, Sort};
use crate::provider::{
    ContentProvider, DatapackProvider, ModProvider, PackProvider, ResourcePackProvider,
    ShaderPackProvider, WorldProvider,
};

/// Modrinth content provider.
///
/// Prefer [`ModrinthProvider::shared`] to reuse a single pooled HTTP client
/// for the lifetime of the process; construct fresh instances only when
/// overriding the user agent or injecting a custom `reqwest::Client`.
#[derive(Debug)]
pub struct ModrinthProvider {
    client: reqwest::Client,
}

impl ModrinthProvider {
    /// Build a provider with a default `reqwest::Client` and the crate's
    /// default User-Agent. Applications embedding this library should
    /// prefer [`Self::with_user_agent`] to brand the UA — Modrinth's
    /// terms of service ask that clients identify themselves distinctly.
    pub fn new() -> Self {
        Self::with_user_agent(super::user_agent::DEFAULT_USER_AGENT)
    }

    /// Build a provider with a caller-supplied User-Agent, e.g.
    /// `"my-launcher/1.2.3 (contact@example.com)"`.
    pub fn with_user_agent(user_agent: &str) -> Self {
        Self {
            client: super::user_agent::client_with_ua(user_agent),
        }
    }

    /// Build a provider using a caller-supplied client. The caller is
    /// responsible for setting a reasonable `User-Agent` on the client.
    pub fn with_client(client: reqwest::Client) -> Self {
        Self { client }
    }

    /// Process-wide shared instance. Constructed on first use.
    pub fn shared() -> &'static Self {
        static INSTANCE: OnceLock<ModrinthProvider> = OnceLock::new();
        INSTANCE.get_or_init(Self::new)
    }
}

impl Default for ModrinthProvider {
    fn default() -> Self {
        Self::new()
    }
}

impl ContentProvider for ModrinthProvider {
    fn platform(&self) -> Platform {
        Platform::Modrinth
    }
}

// ---------------------------------------------------------------------------
// Mods
// ---------------------------------------------------------------------------

impl ModProvider for ModrinthProvider {
    async fn find_mods(
        &self,
        query: Option<&str>,
        sort: Sort,
        filters: &SearchFilters,
        page: u32,
        per_page: u32,
    ) -> Result<Vec<ModItem>> {
        let r = api::search(&self.client, query, sort, ContentType::Mod, filters, page, per_page).await?;
        Ok(r.hits.into_iter().map(mapping::mod_from_hit).collect())
    }

    async fn get_mod(&self, id: &str) -> Result<Option<ModItem>> {
        let p = api::get_project(&self.client, id).await?;
        Ok(p.map(mapping::mod_from_project))
    }
}

// ---------------------------------------------------------------------------
// Modpacks
// ---------------------------------------------------------------------------

impl PackProvider for ModrinthProvider {
    async fn find_packs(
        &self,
        query: Option<&str>,
        sort: Sort,
        filters: &SearchFilters,
        page: u32,
        per_page: u32,
    ) -> Result<Vec<PackItem>> {
        let r = api::search(
            &self.client,
            query,
            sort,
            ContentType::Modpack,
            filters,
            page,
            per_page,
        )
        .await?;
        Ok(r.hits.into_iter().map(mapping::pack_from_hit).collect())
    }

    async fn get_pack(&self, id: &str) -> Result<Option<PackItem>> {
        let p = api::get_project(&self.client, id).await?;
        Ok(p.map(mapping::pack_from_project))
    }
}

// ---------------------------------------------------------------------------
// Datapacks
// ---------------------------------------------------------------------------

impl DatapackProvider for ModrinthProvider {
    async fn find_datapacks(
        &self,
        query: Option<&str>,
        sort: Sort,
        filters: &SearchFilters,
        page: u32,
        per_page: u32,
    ) -> Result<Vec<DatapackItem>> {
        let r = api::search(
            &self.client,
            query,
            sort,
            ContentType::Datapack,
            filters,
            page,
            per_page,
        )
        .await?;
        Ok(r.hits.into_iter().map(mapping::datapack_from_hit).collect())
    }

    async fn get_datapack(&self, id: &str) -> Result<Option<DatapackItem>> {
        let p = api::get_project(&self.client, id).await?;
        Ok(p.map(mapping::datapack_from_project))
    }
}

// ---------------------------------------------------------------------------
// Resource packs
// ---------------------------------------------------------------------------

impl ResourcePackProvider for ModrinthProvider {
    async fn find_resourcepacks(
        &self,
        query: Option<&str>,
        sort: Sort,
        filters: &SearchFilters,
        page: u32,
        per_page: u32,
    ) -> Result<Vec<ResourcePackItem>> {
        let r = api::search(
            &self.client,
            query,
            sort,
            ContentType::ResourcePack,
            filters,
            page,
            per_page,
        )
        .await?;
        Ok(r
            .hits
            .into_iter()
            .map(mapping::resourcepack_from_hit)
            .collect())
    }

    async fn get_resourcepack(&self, id: &str) -> Result<Option<ResourcePackItem>> {
        let p = api::get_project(&self.client, id).await?;
        Ok(p.map(mapping::resourcepack_from_project))
    }
}

// ---------------------------------------------------------------------------
// Shader packs
// ---------------------------------------------------------------------------

impl ShaderPackProvider for ModrinthProvider {
    async fn find_shaderpacks(
        &self,
        query: Option<&str>,
        sort: Sort,
        filters: &SearchFilters,
        page: u32,
        per_page: u32,
    ) -> Result<Vec<ShaderPackItem>> {
        let r = api::search(
            &self.client,
            query,
            sort,
            ContentType::ShaderPack,
            filters,
            page,
            per_page,
        )
        .await?;
        Ok(r
            .hits
            .into_iter()
            .map(mapping::shaderpack_from_hit)
            .collect())
    }

    async fn get_shaderpack(&self, id: &str) -> Result<Option<ShaderPackItem>> {
        let p = api::get_project(&self.client, id).await?;
        Ok(p.map(mapping::shaderpack_from_project))
    }
}

// ---------------------------------------------------------------------------
// Worlds — Modrinth does not host these.
// ---------------------------------------------------------------------------

impl WorldProvider for ModrinthProvider {
    async fn find_worlds(
        &self,
        _query: Option<&str>,
        _sort: Sort,
        _filters: &SearchFilters,
        _page: u32,
        _per_page: u32,
    ) -> Result<Vec<WorldItem>> {
        Err(ContentError::UnsupportedContentType {
            platform: Platform::Modrinth,
            kind: ContentType::World,
        })
    }

    async fn get_world(&self, _id: &str) -> Result<Option<WorldItem>> {
        Err(ContentError::UnsupportedContentType {
            platform: Platform::Modrinth,
            kind: ContentType::World,
        })
    }
}
