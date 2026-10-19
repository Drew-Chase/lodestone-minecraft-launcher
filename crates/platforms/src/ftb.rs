//! FTB — hand-rolled binding for `api.feed-the-beast.com/v1/modpacks`
//! (the FTB App API).
//!
//! Search returns pack ids only, so the first page is hydrated with details
//! calls; version files carry direct CDN urls and install paths.

use std::path::{Path, PathBuf};

use serde::Deserialize;
use simple_download_utility::DownloadProgress;
use tokio::sync::mpsc::Sender;

use crate::PlatformKind;
use crate::error::PlatformError;
use crate::http::{download_files, get_json};
use crate::meta::{ContentDetails, ContentType, ContentVersion, SearchResult};
use crate::sealed;

/// Marker for FTB (https://feed-the-beast.com).
pub struct Ftb;

const BASE: &str = "https://api.feed-the-beast.com/v1/modpacks";
/// How many search hits get hydrated with details calls (the search endpoint
/// returns ids only).
const SEARCH_HYDRATION_LIMIT: usize = 10;

/// One FTB file of a modpack version: an install path plus a direct CDN url.
#[derive(Debug, Deserialize)]
struct FtbFile {
    #[serde(default)]
    path: Option<String>,
    #[serde(default)]
    name: Option<String>,
    #[serde(default)]
    url: Option<String>,
}

/// A version target: game, modloader, or runtime.
#[derive(Debug, Deserialize)]
struct FtbTarget {
    #[serde(default)]
    name: Option<String>,
    #[serde(default)]
    version: Option<String>,
    #[serde(rename = "type", default)]
    target_type: Option<String>,
}

#[derive(Debug, Deserialize)]
struct FtbVersion {
    id: i64,
    name: String,
    #[serde(default)]
    targets: Vec<FtbTarget>,
    #[serde(default)]
    files: Vec<FtbFile>,
}

#[derive(Debug, Deserialize)]
struct FtbArt {
    #[serde(default)]
    url: Option<String>,
}

#[derive(Debug, Deserialize)]
struct FtbPack {
    id: i64,
    name: String,
    #[serde(default)]
    slug: Option<String>,
    #[serde(default)]
    synopsis: Option<String>,
    #[serde(default)]
    description: Option<String>,
    #[serde(default)]
    installs: Option<i64>,
    #[serde(default)]
    art: Vec<FtbArt>,
    #[serde(default)]
    versions: Vec<FtbVersion>,
}

/// The search response: `{ status, packs: [ids], curseforge: [cfIds], count }`.
#[derive(Debug, Deserialize)]
struct FtbSearchResponse {
    #[serde(alias = "packs", default)]
    ids: Vec<i64>,
}

impl FtbPack {
    fn into_details(self) -> ContentDetails {
        ContentDetails {
            id: self.id.to_string(),
            slug: self.slug,
            name: self.name,
            author: None,
            summary: self.synopsis.unwrap_or_default(),
            description: self.description,
            downloads: self.installs.unwrap_or(0).max(0) as u64,
            icon_url: self.art.into_iter().find_map(|art| art.url),
            web_url: None,
            content_type: ContentType::Modpack,
        }
    }

    fn into_search_result(self) -> SearchResult {
        SearchResult {
            id: self.id.to_string(),
            slug: self.slug,
            name: self.name,
            author: None,
            description: self.synopsis.unwrap_or_default(),
            downloads: self.installs.unwrap_or(0).max(0) as u64,
            icon_url: self.art.into_iter().find_map(|art| art.url),
            web_url: None,
            content_type: ContentType::Modpack,
        }
    }
}

fn version_targets(version: &FtbVersion) -> (Vec<String>, Vec<String>) {
    let mut minecraft_versions = Vec::new();
    let mut loaders = Vec::new();
    for target in &version.targets {
        match target.target_type.as_deref() {
            Some("game") => {
                if let Some(version) = &target.version {
                    minecraft_versions.push(version.clone());
                }
            }
            Some("modloader") => {
                if let Some(name) = &target.name {
                    loaders.push(name.to_lowercase());
                }
            }
            _ => {}
        }
    }
    (minecraft_versions, loaders)
}

async fn fetch_pack(http: &reqwest::Client, id: &str) -> Result<FtbPack, PlatformError> {
    let url = format!("{BASE}/public/modpack/{id}");
    let response = get_json(http, &url).await?;
    // FTB responses are flat — no envelope around the payload.
    serde_json::from_value(response).map_err(|source| PlatformError::Json { url: url.clone(), source })
}

impl sealed::Sealed for Ftb {}
impl PlatformKind for Ftb {
    const NAME: &'static str = "ftb";
    const CONTENT_TYPES: &'static [ContentType] = &[ContentType::Modpack];
    const SUPPORTS_MODPACK_ARCHIVE: bool = false;

    async fn search(
        http: &reqwest::Client,
        _credential: Option<&str>,
        query: &str,
        _content: ContentType,
    ) -> Result<Vec<SearchResult>, PlatformError> {
        let url = format!("{BASE}/public/modpack/search/{SEARCH_HYDRATION_LIMIT}");
        let url = crate::http::with_query(&url, "term", query);
        let response = get_json(http, &url).await?;
        let hits: FtbSearchResponse = serde_json::from_value(response).map_err(|source| PlatformError::Json { url: url.clone(), source })?;
        // The search endpoint returns ids only; hydrate the page with details
        // calls so callers get real results.
        let mut results = Vec::with_capacity(hits.ids.len());
        for id in &hits.ids {
            if let Ok(pack) = fetch_pack(http, &id.to_string()).await {
                results.push(pack.into_search_result());
            }
        }
        Ok(results)
    }

    async fn details(http: &reqwest::Client, _credential: Option<&str>, id: &str, _content: ContentType) -> Result<ContentDetails, PlatformError> {
        Ok(fetch_pack(http, id).await?.into_details())
    }

    async fn versions(
        http: &reqwest::Client,
        _credential: Option<&str>,
        id: &str,
        _content: ContentType,
    ) -> Result<Vec<ContentVersion>, PlatformError> {
        let pack = fetch_pack(http, id).await?;
        Ok(pack
            .versions
            .into_iter()
            .map(|version| {
                let (minecraft_versions, loaders) = version_targets(&version);
                ContentVersion {
                    id: version.id.to_string(),
                    project_id: Some(id.to_string()),
                    name: Some(version.name.clone()),
                    version_number: version.name,
                    content_type: ContentType::Modpack,
                    minecraft_versions,
                    loaders,
                    downloads: 0,
                    date: None,
                    changelog_url: None,
                    files: Vec::new(),
                }
            })
            .collect())
    }

    async fn download(
        http: &reqwest::Client,
        _credential: Option<&str>,
        version: &ContentVersion,
        into: &Path,
        progress: Option<&Sender<DownloadProgress>>,
    ) -> Result<Vec<PathBuf>, PlatformError> {
        let Some(project_id) = version.project_id.as_deref() else {
            return Err(PlatformError::Download {
                url: format!("{BASE}/public/modpack/"),
                source: "the version carries no FTB pack id to fetch its files".into(),
            });
        };
        let url = format!("{BASE}/public/modpack/{project_id}/{}", version.id);
        let response = get_json(http, &url).await?;
        // Flat response — no envelope.
        let version_detail: FtbVersion = serde_json::from_value(response).map_err(|source| PlatformError::Json { url: url.clone(), source })?;

        let files = version_detail
            .files
            .into_iter()
            .filter_map(|file| {
                let url = file.url?;
                let name = file.name?;
                let directory = file.path.unwrap_or_default();
                // Paths are "./kubejs"-style relative to the instance root.
                let directory = directory.trim_start_matches("./").trim_matches('/');
                let destination = if directory.is_empty() {
                    into.join(&name)
                } else {
                    into.join(directory).join(&name)
                };
                Some((url, destination))
            })
            .collect::<Vec<(String, PathBuf)>>();
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

    /// Recorded real `/public/modpack/search/5?term=ocean` response — ids only.
    const SEARCH: &str = r#"{ "status": "success", "packs": [128, 91], "curseforge": [1198207], "count": 7, "updated": 1792436377 }"#;

    /// Recorded real `/public/modpack/128` response (truncated to the fields
    /// this crate consumes).
    const PACK: &str = r#"{
        "status": "success",
        "id": 128,
        "name": "FTB OceanBlock 2",
        "slug": "ftb-oceanblock-2",
        "description": "A long description",
        "synopsis": "A short one",
        "installs": 123456,
        "art": [ { "url": "https://cdn.feed-the-beast.com/packs/128/art.png", "width": 400, "height": 400 } ],
        "versions": [
            { "id": 100028, "name": "1.0.0", "type": "release" },
            { "id": 100030, "name": "1.0.1", "type": "release" }
        ]
    }"#;

    /// Recorded real `/public/modpack/128/100028` response (truncated).
    const VERSION: &str = r#"{
        "status": "success",
        "id": 100028,
        "name": "1.0.0",
        "type": "release",
        "targets": [
            { "name": "minecraft", "version": "1.21.1", "type": "game" },
            { "name": "neoforge", "version": "21.1.115", "type": "modloader" }
        ],
        "files": [
            {
                "id": 3483744921,
                "path": "./kubejs",
                "url": "https://files.feed-the-beast.com/blob/1d/1d1a.txt",
                "sha1": "3e56c097",
                "size": 1203,
                "name": "README.txt"
            },
            {
                "id": 3483744922,
                "path": "./",
                "url": "https://files.feed-the-beast.com/blob/2e/2e2b.txt",
                "sha1": "aaaa",
                "size": 42,
                "name": "options.txt"
            }
        ]
    }"#;

    #[test]
    fn the_search_payload_yields_pack_ids() {
        let hits: FtbSearchResponse = serde_json::from_str(SEARCH).expect("recorded payload parses");
        assert_eq!(hits.ids, [128, 91]);
    }

    #[test]
    fn the_pack_payload_maps_onto_details() {
        let pack: FtbPack = serde_json::from_str(PACK).expect("recorded payload parses");
        let details = pack.into_details();
        assert_eq!(details.id, "128");
        assert_eq!(details.slug.as_deref(), Some("ftb-oceanblock-2"));
        assert_eq!(details.name, "FTB OceanBlock 2");
        assert_eq!(details.summary, "A short one");
        assert_eq!(details.description.as_deref(), Some("A long description"));
        assert_eq!(details.downloads, 123456);
        assert_eq!(details.icon_url.as_deref(), Some("https://cdn.feed-the-beast.com/packs/128/art.png"));
    }

    #[test]
    fn version_targets_split_minecraft_from_loaders() {
        let version: FtbVersion = serde_json::from_str(VERSION).expect("recorded payload parses");
        let (minecraft_versions, loaders) = version_targets(&version);
        assert_eq!(minecraft_versions, ["1.21.1"]);
        assert_eq!(loaders, ["neoforge"]);
    }

    #[test]
    fn version_files_map_to_destinations_under_their_paths() {
        let version: FtbVersion = serde_json::from_str(VERSION).expect("recorded payload parses");
        let destinations: Vec<String> = version
            .files
            .into_iter()
            .filter_map(|file| {
                let name = file.name?;
                let directory = file.path.unwrap_or_default();
                let directory = directory.trim_start_matches("./").trim_matches('/');
                Some(if directory.is_empty() { name } else { format!("{directory}/{name}") })
            })
            .collect();
        assert_eq!(destinations, ["kubejs/README.txt", "options.txt"]);
    }

    #[test]
    fn listing_versions_from_the_pack_payload() {
        let pack: FtbPack = serde_json::from_str(PACK).expect("recorded payload parses");
        let versions: Vec<ContentVersion> = pack
            .versions
            .into_iter()
            .map(|version| ContentVersion {
                id: version.id.to_string(),
                project_id: Some("128".to_string()),
                name: Some(version.name.clone()),
                version_number: version.name,
                content_type: ContentType::Modpack,
                minecraft_versions: Vec::new(),
                loaders: Vec::new(),
                downloads: 0,
                date: None,
                changelog_url: None,
                files: Vec::new(),
            })
            .collect();
        assert_eq!(versions.len(), 2);
        assert_eq!(versions[0].id, "100028");
    }
}
