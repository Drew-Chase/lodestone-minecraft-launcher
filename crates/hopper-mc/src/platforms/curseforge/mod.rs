//! CurseForge provider — full implementation against the Core API v1
//! (<https://api.curseforge.com/v1>).
//!
//! # API keys
//!
//! CurseForge requires an `x-api-key` on every request; there is no
//! anonymous mode. Keys are developer secrets issued by
//! <https://console.curseforge.com/> that ship inside client applications.
//!
//! ## Handling the key securely
//!
//! The preferred construction path takes a [`SecretString`] rather than a
//! plain `String`. [`SecretString`] zeros its backing allocation on drop
//! and refuses to appear in `Debug`/`Display` output, so the key does not
//! linger in memory after the provider is destroyed and cannot be
//! accidentally logged.
//!
//! ```no_run
//! use hopper_mc::{CurseForgeProvider, ModProvider, SearchFilters, Sort};
//! use secrecy::SecretString;
//!
//! # async fn example(key: SecretString) -> Result<(), hopper_mc::ContentError> {
//! let cf = CurseForgeProvider::new_with_secret_key(Some(key));
//! let _ = cf.find_mods(Some("sodium"), Sort::Relevance, &SearchFilters::default(), 0, 5).await?;
//! # Ok(()) }
//! ```
//!
//! See `examples/curseforge_with_keyring.rs` for an end-to-end flow that
//! loads the key from the OS credential store (Windows Credential Manager
//! / macOS Keychain / Linux Secret Service) on first run and caches it
//! there for every subsequent run — without the key ever touching an
//! environment variable, a config file, or the process's command line.
//!
//! ## Plain-`String` constructors
//!
//! `new_with_key` / `with_user_agent_and_key` / `with_client_and_key`
//! accept `Option<String>` for ergonomics. They immediately wrap the key
//! into a [`SecretString`] so the provider's internal storage is always
//! zero-on-drop. The caller's own `String`, however, is not — prefer the
//! `_secret_key` variants when you already have a [`SecretString`] in
//! hand.
//!
//! Calls made without a key fail with [`crate::ContentError::BadRequest`]
//! before any network request is issued.

mod api;
mod dto;
mod mapping;

use std::sync::OnceLock;

use secrecy::{ExposeSecret, SecretString};

use crate::error::Result;
use crate::model::{
    DatapackItem, ModItem, PackItem, ProjectVersion, ResourcePackItem, ShaderPackItem, WorldItem,
};
use crate::platform::{ContentType, Platform, SearchFilters, Sort};
use crate::provider::{
    ContentProvider, DatapackProvider, ModProvider, PackProvider, ResourcePackProvider,
    ShaderPackProvider, VersionProvider, WorldProvider,
};

/// CurseForge HTTP provider.
///
/// Because [`SecretString`] refuses to expose its contents through
/// `Debug`, this `Debug` impl is safe to log — the key will print as
/// `SecretBox<str>([REDACTED])`.
#[derive(Debug)]
pub struct CurseForgeProvider {
    client: reqwest::Client,
    api_key: Option<SecretString>,
}

impl CurseForgeProvider {
    /// Build a provider with the crate's default User-Agent and no API
    /// key. Every call will fail with [`crate::ContentError::BadRequest`]
    /// until a key is supplied — prefer [`Self::new_with_secret_key`] or
    /// [`Self::new_with_key`].
    pub fn new() -> Self {
        Self::new_with_secret_key(None)
    }

    /// Build a provider with an explicitly-supplied key as a
    /// [`SecretString`]. This is the preferred keyed constructor — the
    /// key is zeroed from memory when the provider is dropped.
    pub fn new_with_secret_key(api_key: Option<SecretString>) -> Self {
        Self::with_user_agent_and_secret_key(super::user_agent::DEFAULT_USER_AGENT, api_key)
    }

    /// Build a provider with an explicitly-supplied key. The `String` is
    /// immediately wrapped in a [`SecretString`] so the provider's
    /// internal copy is zeroed on drop; however the caller's original
    /// `String` (if any) is not managed by this crate.
    pub fn new_with_key(api_key: Option<String>) -> Self {
        Self::new_with_secret_key(api_key.map(SecretString::from))
    }

    /// Build a provider with a caller-supplied User-Agent and no API key.
    pub fn with_user_agent(user_agent: &str) -> Self {
        Self::with_user_agent_and_secret_key(user_agent, None)
    }

    /// Full-control constructor accepting a [`SecretString`] key.
    pub fn with_user_agent_and_secret_key(
        user_agent: &str,
        api_key: Option<SecretString>,
    ) -> Self {
        Self {
            client: super::user_agent::client_with_ua(user_agent),
            api_key,
        }
    }

    /// Full-control constructor accepting a plain `String` key; the key
    /// is wrapped in a [`SecretString`] before storage.
    pub fn with_user_agent_and_key(user_agent: &str, api_key: Option<String>) -> Self {
        Self::with_user_agent_and_secret_key(user_agent, api_key.map(SecretString::from))
    }

    /// Use a caller-supplied `reqwest::Client` alongside a
    /// [`SecretString`] key. The caller owns the client's configuration
    /// (User-Agent, connection pool, proxy). The `x-api-key` header is
    /// still attached per-request rather than globally so the same client
    /// can be reused across platforms.
    pub fn with_client_and_secret_key(
        client: reqwest::Client,
        api_key: Option<SecretString>,
    ) -> Self {
        Self { client, api_key }
    }

    /// `String`-key variant of [`Self::with_client_and_secret_key`];
    /// wraps the key in a [`SecretString`] before storage.
    pub fn with_client_and_key(client: reqwest::Client, api_key: Option<String>) -> Self {
        Self::with_client_and_secret_key(client, api_key.map(SecretString::from))
    }

    /// Process-wide shared instance, constructed with no API key.
    ///
    /// This exists for parity with the other platform providers and for
    /// the top-level `find_*`/`get_*` dispatch functions, but it is only
    /// useful if CurseForge is reachable without a key (no endpoints
    /// currently are). Keyed access should construct a provider directly
    /// via [`Self::new_with_secret_key`] and call trait methods on it.
    pub fn shared() -> &'static Self {
        static INSTANCE: OnceLock<CurseForgeProvider> = OnceLock::new();
        INSTANCE.get_or_init(Self::new)
    }

    /// Does this provider have a non-empty API key configured? Useful
    /// for surfacing actionable UI *before* making a call that would
    /// fail.
    pub fn has_api_key(&self) -> bool {
        self.api_key
            .as_ref()
            .is_some_and(|s| !s.expose_secret().is_empty())
    }

    /// Expose the key as a short-lived `&str` for attaching to a single
    /// outbound request. The returned reference must not be stored; the
    /// `SecretString` owns the backing allocation and will zero it on
    /// drop.
    fn exposed_key(&self) -> Option<&str> {
        self.api_key.as_ref().map(|s| s.expose_secret())
    }

    /// Resolve download URLs for CurseForge modpack files.
    ///
    /// Iterates `files` that have `project_id` and `file_id` set but empty
    /// `download_urls`, calls the CurseForge API to fetch each file's
    /// metadata, and populates `download_urls`, `size`, and `path`.
    ///
    /// Files with restricted distribution (`download_url: null`) are left
    /// with empty `download_urls`; the caller should handle this gracefully.
    ///
    /// Uses up to 8 concurrent API requests via a semaphore.
    pub async fn resolve_pack_files(
        &self,
        files: &mut [crate::modpack::ModpackFile],
    ) -> Result<()> {
        use tokio::sync::Semaphore;
        use std::sync::Arc;

        let sem = Arc::new(Semaphore::new(8));
        let client = &self.client;
        let key = self.exposed_key();

        let mut handles = Vec::new();

        for (idx, file) in files.iter().enumerate() {
            if !file.download_urls.is_empty() {
                continue;
            }
            let (Some(pid_str), Some(fid_str)) = (&file.project_id, &file.file_id) else {
                continue;
            };
            let Ok(mod_id) = pid_str.parse::<u64>() else {
                continue;
            };
            let Ok(file_id) = fid_str.parse::<u64>() else {
                continue;
            };

            let sem = Arc::clone(&sem);
            let client = client.clone();
            let key = key.map(|k| k.to_string());

            handles.push(tokio::spawn(async move {
                let _permit = sem.acquire().await.unwrap();
                let result = api::get_file(&client, key.as_deref(), mod_id, file_id).await;
                (idx, result)
            }));
        }

        for handle in handles {
            let (idx, result) = handle.await.map_err(|e| {
                crate::error::ContentError::Unexpected(format!("task join error: {e}"))
            })?;
            match result {
                Ok(cf_file) => {
                    let entry = &mut files[idx];
                    if let Some(url) = cf_file.download_url {
                        entry.download_urls.push(url);
                    }
                    entry.size = cf_file.file_length;
                    if entry.path.is_empty() {
                        entry.path = format!("mods/{}", cf_file.file_name);
                    }
                }
                Err(e) => {
                    log::warn!(
                        "Failed to resolve CurseForge file {}/{}: {e}",
                        files[idx].project_id.as_deref().unwrap_or("?"),
                        files[idx].file_id.as_deref().unwrap_or("?"),
                    );
                }
            }
        }

        Ok(())
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
        filters: &SearchFilters,
        page: u32,
        per_page: u32,
    ) -> Result<Vec<ModItem>> {
        let hits = api::search(
            &self.client,
            self.exposed_key(),
            query,
            sort,
            ContentType::Mod,
            filters,
            page,
            per_page,
        )
        .await?;
        Ok(hits.into_iter().map(mapping::mod_from).collect())
    }

    async fn get_mod(&self, id: &str) -> Result<Option<ModItem>> {
        let m = api::get_by_id_or_slug(&self.client, self.exposed_key(), id, mapping::CLASS_MODS).await?;
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
        filters: &SearchFilters,
        page: u32,
        per_page: u32,
    ) -> Result<Vec<PackItem>> {
        let hits = api::search(
            &self.client,
            self.exposed_key(),
            query,
            sort,
            ContentType::Modpack,
            filters,
            page,
            per_page,
        )
        .await?;
        Ok(hits.into_iter().map(mapping::pack_from).collect())
    }

    async fn get_pack(&self, id: &str) -> Result<Option<PackItem>> {
        let m =
            api::get_by_id_or_slug(&self.client, self.exposed_key(), id, mapping::CLASS_MODPACKS).await?;
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
        filters: &SearchFilters,
        page: u32,
        per_page: u32,
    ) -> Result<Vec<DatapackItem>> {
        let hits = api::search(
            &self.client,
            self.exposed_key(),
            query,
            sort,
            ContentType::Datapack,
            filters,
            page,
            per_page,
        )
        .await?;
        Ok(hits.into_iter().map(mapping::datapack_from).collect())
    }

    async fn get_datapack(&self, id: &str) -> Result<Option<DatapackItem>> {
        let m =
            api::get_by_id_or_slug(&self.client, self.exposed_key(), id, mapping::CLASS_DATAPACKS).await?;
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
        filters: &SearchFilters,
        page: u32,
        per_page: u32,
    ) -> Result<Vec<ResourcePackItem>> {
        let hits = api::search(
            &self.client,
            self.exposed_key(),
            query,
            sort,
            ContentType::ResourcePack,
            filters,
            page,
            per_page,
        )
        .await?;
        Ok(hits.into_iter().map(mapping::resourcepack_from).collect())
    }

    async fn get_resourcepack(&self, id: &str) -> Result<Option<ResourcePackItem>> {
        let m = api::get_by_id_or_slug(
            &self.client,
            self.exposed_key(),
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
        filters: &SearchFilters,
        page: u32,
        per_page: u32,
    ) -> Result<Vec<ShaderPackItem>> {
        let hits = api::search(
            &self.client,
            self.exposed_key(),
            query,
            sort,
            ContentType::ShaderPack,
            filters,
            page,
            per_page,
        )
        .await?;
        Ok(hits.into_iter().map(mapping::shaderpack_from).collect())
    }

    async fn get_shaderpack(&self, id: &str) -> Result<Option<ShaderPackItem>> {
        let m =
            api::get_by_id_or_slug(&self.client, self.exposed_key(), id, mapping::CLASS_SHADERS).await?;
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
        filters: &SearchFilters,
        page: u32,
        per_page: u32,
    ) -> Result<Vec<WorldItem>> {
        let hits = api::search(
            &self.client,
            self.exposed_key(),
            query,
            sort,
            ContentType::World,
            filters,
            page,
            per_page,
        )
        .await?;
        Ok(hits.into_iter().map(mapping::world_from).collect())
    }

    async fn get_world(&self, id: &str) -> Result<Option<WorldItem>> {
        let m =
            api::get_by_id_or_slug(&self.client, self.exposed_key(), id, mapping::CLASS_WORLDS).await?;
        Ok(m.map(mapping::world_from))
    }
}

// ---------------------------------------------------------------------------
// Versions
// ---------------------------------------------------------------------------

impl VersionProvider for CurseForgeProvider {
    async fn get_versions(&self, project_id: &str) -> Result<Vec<ProjectVersion>> {
        let mod_id: u64 = project_id
            .parse()
            .map_err(|_| crate::error::ContentError::BadRequest(
                format!("CurseForge project id must be numeric, got: {project_id}"),
            ))?;
        let files = api::get_mod_files(&self.client, self.exposed_key(), mod_id).await?;
        Ok(files.into_iter().map(|f| mapping::version_from_cf(f, project_id)).collect())
    }

    async fn get_version(&self, _version_id: &str) -> Result<Option<ProjectVersion>> {
        // CurseForge uses file IDs, not version IDs. Single-file lookup
        // would require knowing the mod ID too. Return None for now.
        Ok(None)
    }
}
