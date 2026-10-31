//! Modrinth — delegated to ferinth (the gorilla-devs Modrinth API wrapper).
//!
//! Search and the tag lists are hand-rolled: ferinth serializes its facets
//! through `Debug`, which Modrinth answers with zero hits.

use std::path::{Path, PathBuf};
use std::sync::LazyLock;

use ferinth::Ferinth;
use serde::Deserialize;
use simple_download_utility::DownloadProgress;
use tokio::sync::mpsc::Sender;

use crate::PlatformKind;
use crate::error::PlatformError;
use crate::http::{download_files, get_json, with_query};
use crate::meta::{
    ContentDetails, ContentFile, ContentType, ContentVersion, Facet, Facets, SearchPage, SearchQuery, SearchResult, SearchSort, display_name,
};
use crate::sealed;

/// Marker for Modrinth (https://modrinth.com).
pub struct Modrinth;

const SEARCH_BASE: &str = "https://api.modrinth.com/v2/search";
const TAG_BASE: &str = "https://api.modrinth.com/v2/tag";
/// The largest page `/v2/search` returns.
const MAX_PAGE: u32 = 100;

/// ferinth builds and owns its own client (Modrinth expects a descriptive user
/// agent), so it is shared immutably across calls; `Platform::with_client` does
/// not reach this kind.
static API: LazyLock<Ferinth<()>> = LazyLock::new(|| Ferinth::<()>::new("lodestone_platforms", Some(env!("CARGO_PKG_VERSION")), None));

fn web_url(id: &str) -> String {
    format!("https://modrinth.com/project/{id}")
}

/// Modrinth's project-type facet values.
fn project_type_slug(content: ContentType) -> Option<&'static str> {
    match content {
        ContentType::Mod => Some("mod"),
        ContentType::Modpack => Some("modpack"),
        ContentType::Shader => Some("shader"),
        ContentType::ResourcePack => Some("resourcepack"),
        ContentType::DataPack => Some("datapack"),
        ContentType::World => None,
    }
}

/// Modrinth's `index` values, one per [`SearchSort`].
fn sort_index(sort: SearchSort) -> &'static str {
    match sort {
        SearchSort::Relevance => "relevance",
        SearchSort::Popular => "downloads",
        SearchSort::Updated => "updated",
        SearchSort::Newest => "newest",
    }
}

/// The facet groups of a search. Each inner list is one OR group, and the
/// groups are ANDed. The project type always leads, and the facet values must
/// be lowercase (`project_type:mod`), which is why this is hand-rolled.
fn facet_groups(query: &SearchQuery, project_type: &str) -> Vec<Vec<String>> {
    let mut groups = vec![vec![format!("project_type:{project_type}")]];
    if let Some(category) = &query.category {
        groups.push(vec![format!("categories:{category}")]);
    }
    // Modrinth files loaders under categories as well.
    if let Some(loader) = &query.loader {
        groups.push(vec![format!("categories:{loader}")]);
    }
    if let Some(version) = &query.game_version {
        groups.push(vec![format!("versions:{version}")]);
    }
    groups
}

/// The search response of `/v2/search`.
#[derive(Debug, Deserialize)]
struct ModrinthSearchResponse {
    hits: Vec<ModrinthSearchHit>,
    #[serde(default)]
    total_hits: u64,
}

#[derive(Debug, Deserialize)]
struct ModrinthSearchHit {
    #[serde(rename = "project_id")]
    project_id: String,
    #[serde(default)]
    slug: Option<String>,
    title: String,
    author: String,
    description: String,
    #[serde(default)]
    downloads: u64,
    #[serde(rename = "icon_url", default)]
    icon_url: Option<String>,
    #[serde(default)]
    date_modified: Option<String>,
}

/// One row of `/v2/tag/category`.
#[derive(Debug, Deserialize)]
struct ModrinthCategory {
    name: String,
    project_type: String,
    header: String,
}

/// One row of `/v2/tag/loader`.
#[derive(Debug, Deserialize)]
struct ModrinthLoader {
    name: String,
    supported_project_types: Vec<String>,
}

/// One row of `/v2/tag/game_version`.
#[derive(Debug, Deserialize)]
struct ModrinthGameVersion {
    version: String,
    version_type: String,
}

fn search_result(hit: ModrinthSearchHit, content: ContentType) -> SearchResult {
    SearchResult {
        web_url: Some(web_url(&hit.project_id)),
        id: hit.project_id,
        slug: hit.slug,
        name: hit.title,
        author: Some(hit.author),
        description: hit.description,
        downloads: hit.downloads,
        icon_url: hit.icon_url,
        updated: hit.date_modified,
        content_type: content,
    }
}

/// Maps ferinth's error onto `PlatformError` with the url attached.
fn ferinth_error(url: &str, source: ferinth::Error) -> PlatformError {
    match source {
        ferinth::Error::ReqwestError(source) => PlatformError::Network {
            url: url.to_string(),
            source,
        },
        ferinth::Error::JSONError(source) => PlatformError::Json {
            url: url.to_string(),
            source,
        },
        other => PlatformError::Download {
            url: url.to_string(),
            source: Box::new(other),
        },
    }
}

/// GETs one of the tag lists and deserializes its rows.
async fn tag_list<T: serde::de::DeserializeOwned>(http: &reqwest::Client, url: &str) -> Result<Vec<T>, PlatformError> {
    let response = get_json(http, url).await?;
    serde_json::from_value(response).map_err(|source| PlatformError::Json {
        url: url.to_string(),
        source,
    })
}

impl sealed::Sealed for Modrinth {}
impl PlatformKind for Modrinth {
    const NAME: &'static str = "modrinth";
    const CONTENT_TYPES: &'static [ContentType] = &[
        ContentType::Mod,
        ContentType::Modpack,
        ContentType::Shader,
        ContentType::ResourcePack,
        ContentType::DataPack,
    ];
    const SUPPORTS_MODPACK_ARCHIVE: bool = true;

    async fn search(http: &reqwest::Client, _credential: Option<&str>, query: &SearchQuery) -> Result<SearchPage, PlatformError> {
        let Some(project_type) = project_type_slug(query.content) else {
            return Ok(SearchPage::default());
        };
        let facets = serde_json::to_string(&facet_groups(query, project_type)).unwrap_or_else(|_| "[]".to_string());
        let mut url = with_query(SEARCH_BASE, "facets", &facets);
        if !query.text.is_empty() {
            url = with_query(&url, "query", &query.text);
        }
        let url = with_query(&url, "index", sort_index(query.sort));
        let url = with_query(&url, "offset", &query.offset.to_string());
        let url = with_query(&url, "limit", &query.limit.clamp(1, MAX_PAGE).to_string());
        let response = http
            .get(&url)
            .header("User-Agent", concat!("lodestone_platforms/", env!("CARGO_PKG_VERSION")))
            .send()
            .await
            .map_err(|source| PlatformError::Network { url: url.clone(), source })?;
        let status = response.status();
        if !status.is_success() {
            return Err(PlatformError::UnexpectedResponse {
                url: url.clone(),
                status: status.as_u16(),
            });
        }
        let body = response
            .text()
            .await
            .map_err(|source| PlatformError::Network { url: url.clone(), source })?;
        let hits: ModrinthSearchResponse = serde_json::from_str(&body).map_err(|source| PlatformError::Json { url: url.clone(), source })?;
        Ok(SearchPage {
            total: hits.total_hits,
            hits: hits.hits.into_iter().map(|hit| search_result(hit, query.content)).collect(),
        })
    }

    async fn facets(http: &reqwest::Client, _credential: Option<&str>, content: ContentType) -> Result<Facets, PlatformError> {
        let Some(project_type) = project_type_slug(content) else {
            return Ok(Facets::default());
        };
        let categories: Vec<ModrinthCategory> = tag_list(http, &format!("{TAG_BASE}/category")).await?;
        let loaders: Vec<ModrinthLoader> = tag_list(http, &format!("{TAG_BASE}/loader")).await?;
        let versions: Vec<ModrinthGameVersion> = tag_list(http, &format!("{TAG_BASE}/game_version")).await?;

        let loader_names: Vec<String> = loaders
            .into_iter()
            .filter(|loader| loader.supported_project_types.iter().any(|kind| kind == project_type))
            .map(|loader| loader.name)
            .collect();
        Ok(Facets {
            categories: categories
                .into_iter()
                .filter(|category| category.project_type == project_type && category.header == "categories")
                .filter(|category| !loader_names.contains(&category.name))
                .map(|category| Facet {
                    name: display_name(&category.name),
                    id: category.name,
                })
                .collect(),
            loaders: loader_names,
            game_versions: versions
                .into_iter()
                .filter(|version| version.version_type == "release")
                .map(|version| version.version)
                .collect(),
        })
    }

    async fn details(_http: &reqwest::Client, _credential: Option<&str>, id: &str, content: ContentType) -> Result<ContentDetails, PlatformError> {
        let url = web_url(id);
        let project = API.project_get(id).await.map_err(|source| ferinth_error(&url, source))?;
        Ok(ContentDetails {
            id: project.id,
            slug: Some(project.slug),
            name: project.title,
            author: None, // Modrinth resolves authors via the team endpoint; not fetched in this slice
            summary: project.description,
            description: Some(project.body),
            downloads: project.downloads as u64,
            icon_url: project.icon_url.map(|url| url.to_string()),
            web_url: Some(url),
            content_type: content,
        })
    }

    async fn versions(
        _http: &reqwest::Client,
        _credential: Option<&str>,
        id: &str,
        content: ContentType,
    ) -> Result<Vec<ContentVersion>, PlatformError> {
        let url = web_url(id);
        let versions = API.version_list(id).await.map_err(|source| ferinth_error(&url, source))?;
        Ok(versions
            .into_iter()
            .map(|version| ContentVersion {
                id: version.id,
                project_id: Some(version.project_id),
                name: Some(version.name),
                version_number: version.version_number,
                content_type: content,
                minecraft_versions: version.game_versions,
                loaders: version.loaders,
                downloads: version.downloads as u64,
                date: Some(version.date_published.to_rfc3339()),
                changelog_url: None,
                files: version
                    .files
                    .into_iter()
                    .map(|file| ContentFile {
                        name: file.filename,
                        url: file.url.to_string(),
                        sha1: Some(file.hashes.sha1),
                        size: Some(file.size as u64),
                        primary: file.primary,
                    })
                    .collect(),
            })
            .collect())
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

    fn parse_modpack(archive: &Path) -> Result<crate::meta::Modpack, PlatformError> {
        crate::modpack::parse(archive)
    }
}

#[cfg(test)]
#[path = "../tests/modrinth.rs"]
mod tests;
