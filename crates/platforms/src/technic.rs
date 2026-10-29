//! Technic — hand-rolled binding for `api.technicpack.net` (+ Solder builds).
//!
//! Status note: the Technic API currently answers 401 for anonymous callers
//! (verified 2026-10); the binding below follows the documented shapes and the
//! launcher's `X-Build-Data` header convention, and live verification is
//! pending access. Errors surface honestly until then.

use std::path::{Path, PathBuf};

use serde::Deserialize;
use simple_download_utility::DownloadProgress;
use tokio::sync::mpsc::Sender;

use crate::PlatformKind;
use crate::error::PlatformError;
use crate::http::{download_files, get_json, with_query};
use crate::meta::{ContentDetails, ContentType, ContentVersion, SearchResult};
use crate::sealed;

/// Marker for Technic (https://www.technicpack.net).
pub struct Technic;

const BASE: &str = "https://api.technicpack.net";

/// A search hit: a build (pack release) with its pack identity.
#[derive(Debug, Deserialize)]
struct TechnicBuildHit {
    #[serde(default)]
    name: Option<String>,
    slug: String,
    #[serde(default)]
    description: Option<String>,
    #[serde(default)]
    icon_url: Option<String>,
}

#[derive(Debug, Deserialize)]
struct TechnicSearchResponse {
    #[serde(default)]
    builds: Vec<TechnicBuildHit>,
}

/// A pack's details: identity, description, and its build list.
#[derive(Debug, Deserialize)]
struct TechnicPack {
    #[serde(default)]
    name: Option<String>,
    #[serde(default)]
    user: Option<String>,
    #[serde(default)]
    description: Option<String>,
    #[serde(default)]
    builds: Vec<String>,
}

impl TechnicPack {
    fn into_details(self, id: &str) -> ContentDetails {
        ContentDetails {
            id: id.to_string(),
            slug: Some(id.to_string()),
            name: self.name.unwrap_or_else(|| id.to_string()),
            author: self.user,
            summary: self.description.clone().unwrap_or_default(),
            description: self.description,
            downloads: 0,
            icon_url: None,
            web_url: Some(format!("https://www.technicpack.net/modpack/{id}")),
            content_type: ContentType::Modpack,
        }
    }
}

fn build_to_content_version(slug: &str, build: String) -> ContentVersion {
    ContentVersion {
        id: build.clone(),
        project_id: Some(slug.to_string()),
        name: Some(build.clone()),
        version_number: build.clone(),
        content_type: ContentType::Modpack,
        minecraft_versions: Vec::new(),
        loaders: Vec::new(),
        downloads: 0,
        date: None,
        changelog_url: None,
        // The pack zip downloads as one archive from the platform.
        files: vec![crate::meta::ContentFile {
            name: format!("{slug}-{build}.zip"),
            url: format!("{BASE}/modpack/{slug}/download/{build}"),
            sha1: None,
            size: None,
            primary: true,
        }],
    }
}

fn search_to_content_version(hit: TechnicBuildHit) -> SearchResult {
    SearchResult {
        id: hit.slug.clone(),
        slug: Some(hit.slug.clone()),
        name: hit.name.unwrap_or_else(|| hit.slug.clone()),
        author: None,
        description: hit.description.unwrap_or_default(),
        downloads: 0,
        icon_url: hit.icon_url,
        web_url: Some(format!("https://www.technicpack.net/modpack/{}", hit.slug)),
        content_type: ContentType::Modpack,
    }
}

impl sealed::Sealed for Technic {}
impl PlatformKind for Technic {
    const NAME: &'static str = "technic";
    const CONTENT_TYPES: &'static [ContentType] = &[ContentType::Modpack];
    const SUPPORTS_MODPACK_ARCHIVE: bool = false;

    async fn search(
        http: &reqwest::Client,
        _credential: Option<&str>,
        query: &str,
        _content: ContentType,
    ) -> Result<Vec<SearchResult>, PlatformError> {
        let url = with_query(&format!("{BASE}/search/builds"), "q", query);
        let url = with_query(&url, "headers[]", "X-Build-Data");
        let response = get_json(http, &url).await?;
        let hits: TechnicSearchResponse = serde_json::from_value(response).map_err(|source| PlatformError::Json { url: url.clone(), source })?;
        Ok(hits.builds.into_iter().map(search_to_content_version).collect())
    }

    async fn details(http: &reqwest::Client, _credential: Option<&str>, id: &str, _content: ContentType) -> Result<ContentDetails, PlatformError> {
        let url = with_query(&format!("{BASE}/modpack/{id}"), "headers[]", "X-Build-Data");
        let response = get_json(http, &url).await?;
        let pack: TechnicPack = serde_json::from_value(response).map_err(|source| PlatformError::Json { url: url.clone(), source })?;
        Ok(pack.into_details(id))
    }

    async fn versions(
        http: &reqwest::Client,
        _credential: Option<&str>,
        id: &str,
        _content: ContentType,
    ) -> Result<Vec<ContentVersion>, PlatformError> {
        let url = with_query(&format!("{BASE}/modpack/{id}"), "headers[]", "X-Build-Data");
        let response = get_json(http, &url).await?;
        let pack: TechnicPack = serde_json::from_value(response).map_err(|source| PlatformError::Json { url: url.clone(), source })?;
        Ok(pack.builds.into_iter().map(|build| build_to_content_version(id, build)).collect())
    }

    async fn download(
        _http: &reqwest::Client,
        _credential: Option<&str>,
        version: &ContentVersion,
        into: &Path,
        progress: Option<&Sender<DownloadProgress>>,
    ) -> Result<Vec<PathBuf>, PlatformError> {
        // Closure-free (RPITIT `+ Send`): collect destinations with a loop.
        let mut files = Vec::with_capacity(version.files.len());
        for file in &version.files {
            files.push((file.url.clone(), into.join(&file.name)));
        }
        download_files(files, progress).await
    }

    fn parse_modpack(_archive: &Path) -> Result<crate::meta::Modpack, PlatformError> {
        Err(PlatformError::UnsupportedPlatform {
            platform: Self::NAME,
            action: "modpack archive parsing",
        })
    }
}

#[cfg(test)]
#[path = "../tests/technic.rs"]
mod tests;
