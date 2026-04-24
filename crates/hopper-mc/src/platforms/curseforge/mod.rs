//! CurseForge provider — full implementation against the Core API v1
//! (<https://api.curseforge.com/v1>).
//!
//! CurseForge requires an `x-api-key` on every request; there is no
//! anonymous mode. Supply the key in one of three ways:
//!
//! 1. Set the `CURSEFORGE_API_KEY` environment variable before
//!    [`CurseForgeProvider::shared`] / [`CurseForgeProvider::new`] is first
//!    called.
//! 2. Pass it explicitly via [`CurseForgeProvider::new_with_key`] or
//!    [`CurseForgeProvider::with_user_agent_and_key`].
//! 3. Build your own `reqwest::Client` with the header pre-set and hand it
//!    in via [`CurseForgeProvider::with_client_and_key`].
//!
//! Calls made without a key fail with [`ContentError::BadRequest`] before
//! any network request is issued.

mod api;
mod dto;
mod mapping;

use std::sync::OnceLock;

use crate::error::Result;
use crate::model::{
    DatapackItem, ModItem, PackItem, ResourcePackItem, ShaderPackItem, WorldItem,
};
use crate::platform::{ContentType, Platform, Sort};
use crate::provider::{
    ContentProvider, DatapackProvider, ModProvider, PackProvider, ResourcePackProvider,
    ShaderPackProvider, WorldProvider,
};

const API_KEY_ENV: &str = "CURSEFORGE_API_KEY";

#[derive(Debug)]
pub struct CurseForgeProvider {
    client: reqwest::Client,
    api_key: Option<String>,
}

impl CurseForgeProvider {
    /// Build a provider reading the API key from `CURSEFORGE_API_KEY` if set,
    /// with the crate's default User-Agent.
    pub fn new() -> Self {
        Self::new_with_key(std::env::var(API_KEY_ENV).ok())
    }

    /// Build a provider with an explicitly-supplied key (or `None`).
    pub fn new_with_key(api_key: Option<String>) -> Self {
        Self::with_user_agent_and_key(super::user_agent::DEFAULT_USER_AGENT, api_key)
    }

    /// Build a provider with a caller-supplied User-Agent; reads the API key
    /// from `CURSEFORGE_API_KEY` if set.
    pub fn with_user_agent(user_agent: &str) -> Self {
        Self::with_user_agent_and_key(user_agent, std::env::var(API_KEY_ENV).ok())
    }

    /// Full-control constructor.
    pub fn with_user_agent_and_key(user_agent: &str, api_key: Option<String>) -> Self {
        Self {
            client: super::user_agent::client_with_ua(user_agent),
            api_key,
        }
    }

    /// Use a caller-supplied `reqwest::Client` alongside a key. The caller
    /// owns the client's configuration (User-Agent, connection pool, proxy).
    /// The `x-api-key` header is still attached per-request rather than
    /// globally so the client can be reused across platforms.
    pub fn with_client_and_key(client: reqwest::Client, api_key: Option<String>) -> Self {
        Self { client, api_key }
    }

    /// Process-wide shared instance. Constructed on first use; reads the
    /// API key from `CURSEFORGE_API_KEY` at that moment.
    pub fn shared() -> &'static Self {
        static INSTANCE: OnceLock<CurseForgeProvider> = OnceLock::new();
        INSTANCE.get_or_init(Self::new)
    }

    /// Does this provider have an API key configured? Useful for surfacing
    /// actionable UI before making a failing call.
    pub fn has_api_key(&self) -> bool {
        self.api_key.as_deref().is_some_and(|k| !k.is_empty())
    }

    fn key(&self) -> Option<&str> {
        self.api_key.as_deref()
    }
}

impl Default for CurseForgeProvider {
    fn default() -> Self {
        Self::new()
    }
}

impl ContentProvider for CurseForgeProvider {
    fn platform(&self) -> Platform {
        Platform::CurseForge
    }
}

// ---------------------------------------------------------------------------
// Mods
// ---------------------------------------------------------------------------

impl ModProvider for CurseForgeProvider {
    async fn find_mods(
        &self,
        query: Option<&str>,
        sort: Sort,
        page: u32,
        per_page: u32,
    ) -> Result<Vec<ModItem>> {
        let hits = api::search(
            &self.client,
            self.key(),
            query,
            sort,
            ContentType::Mod,
            page,
            per_page,
        )
        .await?;
        Ok(hits.into_iter().map(mapping::mod_from).collect())
    }

    async fn get_mod(&self, id: &str) -> Result<Option<ModItem>> {
        let m = api::get_by_id_or_slug(&self.client, self.key(), id, mapping::CLASS_MODS).await?;
        Ok(m.map(mapping::mod_from))
    }
}

// ---------------------------------------------------------------------------
// Modpacks
// ---------------------------------------------------------------------------

impl PackProvider for CurseForgeProvider {
    async fn find_packs(
        &self,
        query: Option<&str>,
        sort: Sort,
        page: u32,
        per_page: u32,
    ) -> Result<Vec<PackItem>> {
        let hits = api::search(
            &self.client,
            self.key(),
            query,
            sort,
            ContentType::Modpack,
            page,
            per_page,
        )
        .await?;
        Ok(hits.into_iter().map(mapping::pack_from).collect())
    }

    async fn get_pack(&self, id: &str) -> Result<Option<PackItem>> {
        let m =
            api::get_by_id_or_slug(&self.client, self.key(), id, mapping::CLASS_MODPACKS).await?;
        Ok(m.map(mapping::pack_from))
    }
}

// ---------------------------------------------------------------------------
// Datapacks
// ---------------------------------------------------------------------------

impl DatapackProvider for CurseForgeProvider {
    async fn find_datapacks(
        &self,
        query: Option<&str>,
        sort: Sort,
        page: u32,
        per_page: u32,
    ) -> Result<Vec<DatapackItem>> {
        let hits = api::search(
            &self.client,
            self.key(),
            query,
            sort,
            ContentType::Datapack,
            page,
            per_page,
        )
        .await?;
        Ok(hits.into_iter().map(mapping::datapack_from).collect())
    }

    async fn get_datapack(&self, id: &str) -> Result<Option<DatapackItem>> {
        let m =
            api::get_by_id_or_slug(&self.client, self.key(), id, mapping::CLASS_DATAPACKS).await?;
        Ok(m.map(mapping::datapack_from))
    }
}

// ---------------------------------------------------------------------------
// Resource packs
// ---------------------------------------------------------------------------

impl ResourcePackProvider for CurseForgeProvider {
    async fn find_resourcepacks(
        &self,
        query: Option<&str>,
        sort: Sort,
        page: u32,
        per_page: u32,
    ) -> Result<Vec<ResourcePackItem>> {
        let hits = api::search(
            &self.client,
            self.key(),
            query,
            sort,
            ContentType::ResourcePack,
            page,
            per_page,
        )
        .await?;
        Ok(hits.into_iter().map(mapping::resourcepack_from).collect())
    }

    async fn get_resourcepack(&self, id: &str) -> Result<Option<ResourcePackItem>> {
        let m = api::get_by_id_or_slug(
            &self.client,
            self.key(),
            id,
            mapping::CLASS_RESOURCEPACKS,
        )
        .await?;
        Ok(m.map(mapping::resourcepack_from))
    }
}

// ---------------------------------------------------------------------------
// Shader packs
// ---------------------------------------------------------------------------

impl ShaderPackProvider for CurseForgeProvider {
    async fn find_shaderpacks(
        &self,
        query: Option<&str>,
        sort: Sort,
        page: u32,
        per_page: u32,
    ) -> Result<Vec<ShaderPackItem>> {
        let hits = api::search(
            &self.client,
            self.key(),
            query,
            sort,
            ContentType::ShaderPack,
            page,
            per_page,
        )
        .await?;
        Ok(hits.into_iter().map(mapping::shaderpack_from).collect())
    }

    async fn get_shaderpack(&self, id: &str) -> Result<Option<ShaderPackItem>> {
        let m =
            api::get_by_id_or_slug(&self.client, self.key(), id, mapping::CLASS_SHADERS).await?;
        Ok(m.map(mapping::shaderpack_from))
    }
}

// ---------------------------------------------------------------------------
// Worlds
// ---------------------------------------------------------------------------

impl WorldProvider for CurseForgeProvider {
    async fn find_worlds(
        &self,
        query: Option<&str>,
        sort: Sort,
        page: u32,
        per_page: u32,
    ) -> Result<Vec<WorldItem>> {
        let hits = api::search(
            &self.client,
            self.key(),
            query,
            sort,
            ContentType::World,
            page,
            per_page,
        )
        .await?;
        Ok(hits.into_iter().map(mapping::world_from).collect())
    }

    async fn get_world(&self, id: &str) -> Result<Option<WorldItem>> {
        let m =
            api::get_by_id_or_slug(&self.client, self.key(), id, mapping::CLASS_WORLDS).await?;
        Ok(m.map(mapping::world_from))
    }
}
