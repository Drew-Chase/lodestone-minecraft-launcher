//! FTB (Feed The Beast) provider — **skeleton**, modpacks only.
//!
//! FTB's app API lives at `https://api.modpacks.ch/public/`. Fill in the
//! bodies when wiring the real integration.

use std::sync::OnceLock;

use crate::error::{ContentError, Result};
use crate::model::PackItem;
use crate::platform::{Platform, Sort};
use crate::provider::{ContentProvider, PackProvider};

#[derive(Debug)]
pub struct FtbProvider {
    #[allow(dead_code)]
    client: reqwest::Client,
}

impl FtbProvider {
    pub fn new() -> Self {
        Self::with_user_agent(super::user_agent::DEFAULT_USER_AGENT)
    }

    pub fn with_user_agent(user_agent: &str) -> Self {
        Self {
            client: super::user_agent::client_with_ua(user_agent),
        }
    }

    pub fn shared() -> &'static Self {
        static INSTANCE: OnceLock<FtbProvider> = OnceLock::new();
        INSTANCE.get_or_init(Self::new)
    }
}

impl Default for FtbProvider {
    fn default() -> Self {
        Self::new()
    }
}

impl ContentProvider for FtbProvider {
    fn platform(&self) -> Platform {
        Platform::Ftb
    }
}

impl PackProvider for FtbProvider {
    async fn find_packs(
        &self,
        _query: Option<&str>,
        _sort: Sort,
        _page: u32,
        _per_page: u32,
    ) -> Result<Vec<PackItem>> {
        Err(ContentError::NotImplemented(Platform::Ftb))
    }
    async fn get_pack(&self, _id: &str) -> Result<Option<PackItem>> {
        Err(ContentError::NotImplemented(Platform::Ftb))
    }
}
