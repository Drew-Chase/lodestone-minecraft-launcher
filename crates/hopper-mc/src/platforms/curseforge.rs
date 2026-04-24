//! CurseForge provider — **skeleton**. Every call returns
//! [`ContentError::NotImplemented`]. Fill in the bodies when the CurseForge
//! API key / endpoint integration is ready; signatures are final.
//!
//! CurseForge's public API lives at `https://api.curseforge.com/v1/` and
//! requires an `x-api-key` header. The provider's `new_with_key` constructor
//! is already plumbed so implementation is body-only.

use std::sync::OnceLock;

use crate::error::{ContentError, Result};
use crate::model::{
    DatapackItem, ModItem, PackItem, ResourcePackItem, ShaderPackItem, WorldItem,
};
use crate::platform::{Platform, Sort};
use crate::provider::{
    ContentProvider, DatapackProvider, ModProvider, PackProvider, ResourcePackProvider,
    ShaderPackProvider, WorldProvider,
};

#[derive(Debug)]
pub struct CurseForgeProvider {
    #[allow(dead_code)] // used once the skeleton is fleshed out
    client: reqwest::Client,
    #[allow(dead_code)]
    api_key: Option<String>,
}

impl CurseForgeProvider {
    pub fn new() -> Self {
        Self::new_with_key(None)
    }

    pub fn new_with_key(api_key: Option<String>) -> Self {
        Self::with_user_agent_and_key(super::user_agent::DEFAULT_USER_AGENT, api_key)
    }

    pub fn with_user_agent(user_agent: &str) -> Self {
        Self::with_user_agent_and_key(user_agent, None)
    }

    pub fn with_user_agent_and_key(user_agent: &str, api_key: Option<String>) -> Self {
        Self {
            client: super::user_agent::client_with_ua(user_agent),
            api_key,
        }
    }

    pub fn shared() -> &'static Self {
        static INSTANCE: OnceLock<CurseForgeProvider> = OnceLock::new();
        INSTANCE.get_or_init(Self::new)
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

fn ni<T>() -> Result<T> {
    Err(ContentError::NotImplemented(Platform::CurseForge))
}

impl ModProvider for CurseForgeProvider {
    async fn find_mods(
        &self,
        _query: Option<&str>,
        _sort: Sort,
        _page: u32,
        _per_page: u32,
    ) -> Result<Vec<ModItem>> {
        ni()
    }
    async fn get_mod(&self, _id: &str) -> Result<Option<ModItem>> {
        ni()
    }
}

impl PackProvider for CurseForgeProvider {
    async fn find_packs(
        &self,
        _query: Option<&str>,
        _sort: Sort,
        _page: u32,
        _per_page: u32,
    ) -> Result<Vec<PackItem>> {
        ni()
    }
    async fn get_pack(&self, _id: &str) -> Result<Option<PackItem>> {
        ni()
    }
}

impl DatapackProvider for CurseForgeProvider {
    async fn find_datapacks(
        &self,
        _query: Option<&str>,
        _sort: Sort,
        _page: u32,
        _per_page: u32,
    ) -> Result<Vec<DatapackItem>> {
        ni()
    }
    async fn get_datapack(&self, _id: &str) -> Result<Option<DatapackItem>> {
        ni()
    }
}

impl ResourcePackProvider for CurseForgeProvider {
    async fn find_resourcepacks(
        &self,
        _query: Option<&str>,
        _sort: Sort,
        _page: u32,
        _per_page: u32,
    ) -> Result<Vec<ResourcePackItem>> {
        ni()
    }
    async fn get_resourcepack(&self, _id: &str) -> Result<Option<ResourcePackItem>> {
        ni()
    }
}

impl ShaderPackProvider for CurseForgeProvider {
    async fn find_shaderpacks(
        &self,
        _query: Option<&str>,
        _sort: Sort,
        _page: u32,
        _per_page: u32,
    ) -> Result<Vec<ShaderPackItem>> {
        ni()
    }
    async fn get_shaderpack(&self, _id: &str) -> Result<Option<ShaderPackItem>> {
        ni()
    }
}

impl WorldProvider for CurseForgeProvider {
    async fn find_worlds(
        &self,
        _query: Option<&str>,
        _sort: Sort,
        _page: u32,
        _per_page: u32,
    ) -> Result<Vec<WorldItem>> {
        ni()
    }
    async fn get_world(&self, _id: &str) -> Result<Option<WorldItem>> {
        ni()
    }
}
