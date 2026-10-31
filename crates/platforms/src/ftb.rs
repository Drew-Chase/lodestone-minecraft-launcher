//! FTB — hand-rolled binding for `api.feed-the-beast.com/v1/modpacks`
//! (the FTB App API).
//!
//! Search returns pack ids only, so each page is hydrated with details calls;
//! version files carry direct CDN urls and install paths.

use std::path::{Path, PathBuf};

use futures_util::future::join_all;
use serde::Deserialize;
use simple_download_utility::DownloadProgress;
use tokio::sync::mpsc::Sender;

use crate::PlatformKind;
use crate::error::PlatformError;
use crate::http::{HYDRATION_CHUNK, download_files, get_json};
use crate::meta::{ContentDetails, ContentType, ContentVersion, Facets, SearchPage, SearchQuery, SearchResult};
use crate::sealed;

/// Marker for FTB (https://feed-the-beast.com).
pub struct Ftb;

const BASE: &str = "https://api.feed-the-beast.com/v1/modpacks";
/// The most ids one search asks for. The endpoints page by count rather than
/// by offset, so a page past this many hits is not reachable.
const MAX_SEARCH: usize = 100;

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
    #[serde(default)]
    width: Option<u64>,
    #[serde(default)]
    height: Option<u64>,
}

/// The largest picture a pack offers. FTB lists several sizes, and the biggest reads best as a banner.
fn largest_art(art: Vec<FtbArt>) -> Option<String> {
    art.into_iter()
        .filter(|art| art.url.is_some())
        .max_by_key(|art| art.width.unwrap_or(0) * art.height.unwrap_or(0))
        .and_then(|art| art.url)
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
    /// How many packs match the search in all.
    #[serde(default)]
    count: Option<u64>,
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
            icon_url: largest_art(self.art),
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
            icon_url: largest_art(self.art),
            web_url: None,
            updated: None,
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

    async fn search(http: &reqwest::Client, _credential: Option<&str>, query: &SearchQuery) -> Result<SearchPage, PlatformError> {
        // The endpoints take a count rather than an offset, so ask for every id
        // up to the end of this page and keep the slice that belongs to it.
        let wanted = (query.offset as usize + query.limit as usize).clamp(1, MAX_SEARCH);
        let url = if query.text.is_empty() {
            format!("{BASE}/public/modpack/popular/installs/{wanted}")
        } else {
            crate::http::with_query(&format!("{BASE}/public/modpack/search/{wanted}"), "term", &query.text)
        };
        let response = get_json(http, &url).await?;
        let found: FtbSearchResponse = serde_json::from_value(response).map_err(|source| PlatformError::Json { url: url.clone(), source })?;
        let total = found.count.unwrap_or(found.ids.len() as u64);
        let page: Vec<i64> = found.ids.into_iter().skip(query.offset as usize).take(query.limit as usize).collect();
        // The endpoints return ids only, so the page is hydrated with details calls, a few at a time.
        let ids: Vec<String> = page.iter().map(i64::to_string).collect();
        let mut hits = Vec::with_capacity(page.len());
        for chunk in ids.chunks(HYDRATION_CHUNK) {
            let mut calls = Vec::with_capacity(chunk.len());
            for id in chunk {
                calls.push(fetch_pack(http, id));
            }
            for pack in join_all(calls).await.into_iter().flatten() {
                hits.push(pack.into_search_result());
            }
        }
        Ok(SearchPage { hits, total })
    }

    async fn facets(_http: &reqwest::Client, _credential: Option<&str>, _content: ContentType) -> Result<Facets, PlatformError> {
        Ok(Facets::default())
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

/// The loader a version's modloader target names, as `lodestone_loaders` names it.
fn loader_of(version: &FtbVersion) -> Option<&'static str> {
    let name = version
        .targets
        .iter()
        .find(|target| target.target_type.as_deref() == Some("modloader"))?
        .name
        .as_deref()?
        .to_lowercase();
    ["fabric", "forge", "neoforge", "quilt"].into_iter().find(|known| *known == name)
}

/// The loader version a version's modloader target pins.
fn loader_version_of(version: &FtbVersion) -> Option<String> {
    version
        .targets
        .iter()
        .find(|target| target.target_type.as_deref() == Some("modloader"))
        .and_then(|target| target.version.clone())
}

/// The installable modpack of one FTB version: its files at their install paths on FTB's CDN, and the
/// Minecraft version, loader and loader version it targets. Reached through `Platform::<Ftb>::modpack`.
pub(crate) async fn modpack(http: &reqwest::Client, version: &ContentVersion) -> Result<crate::meta::Modpack, PlatformError> {
    let Some(project_id) = version.project_id.as_deref() else {
        return Err(PlatformError::Download {
            url: format!("{BASE}/public/modpack/"),
            source: "the version carries no FTB pack id to build the modpack from".into(),
        });
    };
    let pack = fetch_pack(http, project_id).await?;
    let url = format!("{BASE}/public/modpack/{project_id}/{}", version.id);
    let response = get_json(http, &url).await?;
    let detail: FtbVersion = serde_json::from_value(response).map_err(|source| PlatformError::Json { url: url.clone(), source })?;
    let (minecraft_versions, _) = version_targets(&detail);
    let files = detail
        .files
        .iter()
        .filter_map(|file| {
            let url = file.url.clone()?;
            let name = file.name.clone()?;
            let directory = file.path.as_deref().unwrap_or_default().trim_start_matches("./").trim_matches('/');
            let path = if directory.is_empty() { name } else { format!("{directory}/{name}") };
            Some(crate::meta::ModpackFile {
                path,
                url,
                sha1: None,
                size: None,
                project_id: None,
                file_id: None,
            })
        })
        .collect();
    Ok(crate::meta::Modpack {
        name: pack.name,
        version: Some(version.version_number.clone()),
        minecraft: minecraft_versions.into_iter().next().unwrap_or_default(),
        loader: loader_of(&detail),
        loader_version: loader_version_of(&detail),
        files,
        overrides: None,
        overrides_skip: Vec::new(),
        archives: Vec::new(),
    })
}

#[cfg(test)]
#[path = "../tests/ftb.rs"]
mod tests;
