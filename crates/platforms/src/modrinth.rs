//! Modrinth — delegated to ferinth (the gorilla-devs Modrinth API wrapper).

use std::path::{Path, PathBuf};
use std::sync::LazyLock;

use ferinth::Ferinth;
use serde::Deserialize;
use simple_download_utility::DownloadProgress;
use tokio::sync::mpsc::Sender;

use crate::PlatformKind;
use crate::error::PlatformError;
use crate::http::{download_files, with_query};
use crate::meta::{ContentDetails, ContentFile, ContentType, ContentVersion, SearchResult};
use crate::sealed;
/// Marker for Modrinth (https://modrinth.com).
pub struct Modrinth;

const SEARCH_BASE: &str = "https://api.modrinth.com/v2/search";

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

/// The search response of `/v2/search`.
///
/// Hand-rolled instead of ferinth's `search`: ferinth serializes its
/// `ProjectType` facet through `Debug` (`project_type:Mod`), which Modrinth
/// answers with zero hits — the facet must be lowercase (`project_type:mod`).
#[derive(Debug, Deserialize)]
struct ModrinthSearchResponse {
    hits: Vec<ModrinthSearchHit>,
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

    async fn search(
        http: &reqwest::Client,
        _credential: Option<&str>,
        query: &str,
        content: ContentType,
    ) -> Result<Vec<SearchResult>, PlatformError> {
        let Some(project_type) = project_type_slug(content) else {
            return Ok(Vec::new());
        };
        let url = with_query(SEARCH_BASE, "query", query);
        let url = with_query(&url, "facets", &format!("[[\"project_type:{project_type}\"]]"));
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

        let results = hits.hits.into_iter().map(|hit| SearchResult {
            id: hit.project_id,
            slug: hit.slug,
            name: hit.title,
            author: Some(hit.author),
            description: hit.description,
            downloads: hit.downloads,
            icon_url: hit.icon_url,
            web_url: None,
            content_type: content,
        });
        Ok(results.collect())
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
