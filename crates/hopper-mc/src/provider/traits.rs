//! Provider trait hierarchy.
//!
//! Every platform implements [`ContentProvider`] plus one trait per content
//! kind it can serve. Platforms that don't serve a given kind simply don't
//! implement that trait, and the top-level dispatch functions return
//! [`ContentError::UnsupportedContentType`](crate::error::ContentError::UnsupportedContentType).
//!
//! Async fns in traits are used directly (stable since 1.75; the workspace
//! is on edition 2024), so no `async-trait` indirection is needed.

use crate::error::Result;
use crate::model::{
    DatapackItem, ModItem, PackItem, ResourcePackItem, ShaderPackItem, WorldItem,
};
use crate::platform::{Platform, Sort};

/// Base trait: every provider identifies the platform it speaks to.
pub trait ContentProvider: Send + Sync {
    fn platform(&self) -> Platform;
}

/// A provider that can serve mods.
pub trait ModProvider: ContentProvider {
    fn find_mods(
        &self,
        query: Option<&str>,
        sort: Sort,
        page: u32,
        per_page: u32,
    ) -> impl std::future::Future<Output = Result<Vec<ModItem>>> + Send;

    fn get_mod(
        &self,
        id: &str,
    ) -> impl std::future::Future<Output = Result<Option<ModItem>>> + Send;
}

/// A provider that can serve modpacks.
pub trait PackProvider: ContentProvider {
    fn find_packs(
        &self,
        query: Option<&str>,
        sort: Sort,
        page: u32,
        per_page: u32,
    ) -> impl std::future::Future<Output = Result<Vec<PackItem>>> + Send;

    fn get_pack(
        &self,
        id: &str,
    ) -> impl std::future::Future<Output = Result<Option<PackItem>>> + Send;
}

/// A provider that can serve datapacks.
pub trait DatapackProvider: ContentProvider {
    fn find_datapacks(
        &self,
        query: Option<&str>,
        sort: Sort,
        page: u32,
        per_page: u32,
    ) -> impl std::future::Future<Output = Result<Vec<DatapackItem>>> + Send;

    fn get_datapack(
        &self,
        id: &str,
    ) -> impl std::future::Future<Output = Result<Option<DatapackItem>>> + Send;
}

/// A provider that can serve resource packs.
pub trait ResourcePackProvider: ContentProvider {
    fn find_resourcepacks(
        &self,
        query: Option<&str>,
        sort: Sort,
        page: u32,
        per_page: u32,
    ) -> impl std::future::Future<Output = Result<Vec<ResourcePackItem>>> + Send;

    fn get_resourcepack(
        &self,
        id: &str,
    ) -> impl std::future::Future<Output = Result<Option<ResourcePackItem>>> + Send;
}

/// A provider that can serve shader packs.
pub trait ShaderPackProvider: ContentProvider {
    fn find_shaderpacks(
        &self,
        query: Option<&str>,
        sort: Sort,
        page: u32,
        per_page: u32,
    ) -> impl std::future::Future<Output = Result<Vec<ShaderPackItem>>> + Send;

    fn get_shaderpack(
        &self,
        id: &str,
    ) -> impl std::future::Future<Output = Result<Option<ShaderPackItem>>> + Send;
}

/// A provider that can serve downloadable worlds / saves.
pub trait WorldProvider: ContentProvider {
    fn find_worlds(
        &self,
        query: Option<&str>,
        sort: Sort,
        page: u32,
        per_page: u32,
    ) -> impl std::future::Future<Output = Result<Vec<WorldItem>>> + Send;

    fn get_world(
        &self,
        id: &str,
    ) -> impl std::future::Future<Output = Result<Option<WorldItem>>> + Send;
}
