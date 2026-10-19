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
mod tests {
    use super::*;

    /// The documented search shape (currently 401-gated live; shape from the
    /// historical public API).
    const SEARCH: &str = r#"{
        "builds": [
            { "id": 1, "name": "Hexxit", "slug": "hexxit", "description": "Adventure awaits", "icon_url": "https://icon.test/hexxit.png" }
        ]
    }"#;

    /// The documented pack shape (currently 401-gated live).
    const PACK: &str = r#"{
        "name": "Hexxit",
        "user": "Technic",
        "description": "Adventure awaits",
        "recommended": "1.0.1",
        "latest": "1.0.2",
        "builds": ["1.0.1", "1.0.2"]
    }"#;

    #[test]
    fn the_search_shape_maps_onto_search_results() {
        let hits: TechnicSearchResponse = serde_json::from_str(SEARCH).expect("documented shape parses");
        let results: Vec<SearchResult> = hits.builds.into_iter().map(search_to_content_version).collect();
        assert_eq!(results.len(), 1);
        assert_eq!(results[0].id, "hexxit");
        assert_eq!(results[0].name, "Hexxit");
        assert_eq!(results[0].web_url.as_deref(), Some("https://www.technicpack.net/modpack/hexxit"));
    }

    #[test]
    fn the_pack_shape_maps_onto_details_and_versions() {
        let pack: TechnicPack = serde_json::from_str(PACK).expect("documented shape parses");
        let details = pack.into_details("hexxit");
        assert_eq!(details.name, "Hexxit");
        assert_eq!(details.author.as_deref(), Some("Technic"));
        assert_eq!(details.web_url.as_deref(), Some("https://www.technicpack.net/modpack/hexxit"));
    }

    #[test]
    fn builds_map_onto_versions_with_download_archives() {
        let pack: TechnicPack = serde_json::from_str(PACK).expect("documented shape parses");
        let versions: Vec<ContentVersion> = pack.builds.into_iter().map(|build| build_to_content_version("hexxit", build)).collect();
        assert_eq!(versions.len(), 2);
        assert_eq!(versions[0].version_number, "1.0.1");
        assert_eq!(versions[0].files[0].url, "https://api.technicpack.net/modpack/hexxit/download/1.0.1");
        assert_eq!(versions[0].files[0].name, "hexxit-1.0.1.zip");
    }
}
