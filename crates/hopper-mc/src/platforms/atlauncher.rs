//! AT Launcher provider — **skeleton**, modpacks only.
//!
//! AT Launcher exposes its pack index at `https://api.atlauncher.com/v2/packs/`.
//! Fill in the bodies when wiring the real integration.

use std::sync::OnceLock;

use crate::error::{ContentError, Result};
use crate::model::PackItem;
use crate::platform::{Platform, Sort};
use crate::provider::{ContentProvider, PackProvider};

#[derive(Debug)]
pub struct AtLauncherProvider {
    #[allow(dead_code)]
    client: reqwest::Client,
}

impl AtLauncherProvider {
    pub fn new() -> Self {
        Self::with_user_agent(super::user_agent::DEFAULT_USER_AGENT)
    }

    pub fn with_user_agent(user_agent: &str) -> Self {
        Self {
            client: super::user_agent::client_with_ua(user_agent),
        }
    }

    pub fn shared() -> &'static Self {
        static INSTANCE: OnceLock<AtLauncherProvider> = OnceLock::new();
        INSTANCE.get_or_init(Self::new)
    }
}

impl Default for AtLauncherProvider {
    fn default() -> Self {
        Self::new()
    }
}

impl ContentProvider for AtLauncherProvider {
    fn platform(&self) -> Platform {
        Platform::AtLauncher
    }
}

impl PackProvider for AtLauncherProvider {
    async fn find_packs(
        &self,
        _query: Option<&str>,
        _sort: Sort,
        _page: u32,
        _per_page: u32,
    ) -> Result<Vec<PackItem>> {
        Err(ContentError::NotImplemented(Platform::AtLauncher))
    }
    async fn get_pack(&self, _id: &str) -> Result<Option<PackItem>> {
        Err(ContentError::NotImplemented(Platform::AtLauncher))
    }
}
