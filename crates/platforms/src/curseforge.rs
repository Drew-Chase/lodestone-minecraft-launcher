//! CurseForge — hand-rolled binding for `api.curseforge.com/v1`.
//!
//! `furse` was considered for this kind but wraps no search, which would have
//! split the kind across furse (reqwest 0.12) and hand-rolled search anyway;
//! a coherent thin binding on the workspace's reqwest 0.13 was chosen instead
//! (DESIGN.md §2).
//!
//! Requires an API key: use `Platform::<CurseForge>::with_api_key`.

use std::path::{Path, PathBuf};

use serde::Deserialize;
use simple_download_utility::DownloadProgress;
use tokio::sync::mpsc::Sender;

use crate::PlatformKind;
use crate::error::PlatformError;
use crate::http::{download_files, get_json, with_query};
use crate::meta::{ContentDetails, ContentFile, ContentType, ContentVersion, Facet, Facets, SearchPage, SearchQuery, SearchResult, SearchSort};
use crate::sealed;

/// Marker for CurseForge (https://www.curseforge.com). Requires an API key —
/// see [`Platform::<CurseForge>::with_api_key`](crate::Platform::with_api_key).
pub struct CurseForge;

const BASE: &str = "https://api.curseforge.com/v1";
/// CurseForge's game id for Minecraft.
const MINECRAFT_GAME_ID: u32 = 432;
/// The largest page `/mods/search` returns.
const MAX_PAGE: u32 = 50;

/// CurseForge's top-level content classes that map onto [`ContentType`]s.
fn class_id(content: ContentType) -> Option<u32> {
    match content {
        ContentType::Mod => Some(6),
        ContentType::Modpack => Some(4471),
        ContentType::ResourcePack => Some(12),
        ContentType::World => Some(17),
        ContentType::Shader | ContentType::DataPack => None,
    }
}

/// CurseForge's `modLoader` int values → loader names.
fn loader_name(mod_loader: Option<i64>) -> Vec<String> {
    match mod_loader {
        Some(1) => vec!["forge".to_string()],
        Some(4) => vec!["neoforge".to_string()],
        Some(5) => vec!["fabric".to_string()],
        Some(6) => vec!["quilt".to_string()],
        _ => Vec::new(),
    }
}

/// The loader names a search filters by, paired with CurseForge's `modLoaderType`
/// values. The inverse of [`loader_name`].
const LOADERS: [(&str, u32); 4] = [("forge", 1), ("neoforge", 4), ("fabric", 5), ("quilt", 6)];

/// CurseForge's `sortField` values, one per [`SearchSort`].
fn sort_field(sort: SearchSort) -> u32 {
    match sort {
        SearchSort::Relevance => 1,
        SearchSort::Popular => 2,
        SearchSort::Updated => 3,
        SearchSort::Newest => 11,
    }
}

/// Builds a client carrying the API key header plus the full url for `path`.
/// Fails up front when the handle has no credential.
fn request(credential: Option<&str>, path: &str) -> Result<(String, reqwest::Client), PlatformError> {
    let Some(key) = credential else {
        return Err(PlatformError::UnexpectedResponse {
            url: format!("{BASE}{path}"),
            status: 401,
        });
    };
    let mut headers = reqwest::header::HeaderMap::new();
    let value = reqwest::header::HeaderValue::from_str(key).map_err(|_| PlatformError::Download {
        url: format!("{BASE}{path}"),
        source: "the CurseForge API key contains invalid header characters".into(),
    })?;
    headers.insert(reqwest::header::HeaderName::from_static("x-api-key"), value);
    let client = reqwest::Client::builder()
        .default_headers(headers)
        .user_agent(concat!("lodestone_platforms/", env!("CARGO_PKG_VERSION")))
        .build()
        .map_err(|source| PlatformError::Download {
            url: BASE.to_string(),
            source: Box::new(source),
        })?;
    Ok((format!("{BASE}{path}"), client))
}

/// A CurseForge project (mod, modpack, resource pack, world).
#[derive(Debug, Deserialize)]
struct CfMod {
    id: i64,
    name: String,
    slug: String,
    summary: String,
    #[serde(rename = "downloadCount", default)]
    download_count: u64,
    #[serde(rename = "dateModified", default)]
    date_modified: Option<String>,
    #[serde(default)]
    authors: Vec<CfAuthor>,
    #[serde(default)]
    logo: Option<CfLogo>,
    #[serde(default)]
    links: Option<CfLinks>,
}

#[derive(Debug, Deserialize)]
struct CfAuthor {
    name: String,
}

#[derive(Debug, Deserialize)]
struct CfLogo {
    #[serde(rename = "thumbnailUrl", default)]
    thumbnail_url: Option<String>,
    #[serde(default)]
    url: Option<String>,
}

#[derive(Debug, Deserialize)]
struct CfLinks {
    #[serde(rename = "websiteUrl", default)]
    website_url: Option<String>,
}

#[derive(Debug, Deserialize)]
struct CfPagination {
    #[serde(rename = "totalCount", default)]
    total_count: u64,
}

#[derive(Debug, Deserialize)]
struct CfModList {
    data: Vec<CfMod>,
    #[serde(default)]
    pagination: Option<CfPagination>,
}

#[derive(Debug, Deserialize)]
struct CfModEnvelope {
    data: CfMod,
}

#[derive(Debug, Deserialize)]
struct CfDescription {
    data: String,
}

/// One CurseForge category: its numeric id and its name.
#[derive(Debug, Deserialize)]
struct CfCategory {
    id: i64,
    name: String,
}

#[derive(Debug, Deserialize)]
struct CfCategoryList {
    data: Vec<CfCategory>,
}

/// One Minecraft version row of `/minecraft/version`.
#[derive(Debug, Deserialize)]
struct CfGameVersion {
    #[serde(rename = "versionString")]
    version_string: String,
}

#[derive(Debug, Deserialize)]
struct CfGameVersionList {
    data: Vec<CfGameVersion>,
}

/// A CurseForge file — one release of a project.
#[derive(Debug, Deserialize)]
struct CfFile {
    id: i64,
    #[serde(rename = "displayName", default)]
    display_name: Option<String>,
    #[serde(rename = "fileName")]
    file_name: String,
    #[serde(rename = "downloadUrl", default)]
    download_url: Option<String>,
    #[serde(rename = "fileLength", default)]
    file_length: u64,
    #[serde(default)]
    hashes: Vec<CfHash>,
    #[serde(rename = "gameVersions", default)]
    game_versions: Vec<String>,
    #[serde(rename = "modLoader", default)]
    mod_loader: Option<i64>,
    #[serde(rename = "fileDate", default)]
    file_date: Option<String>,
}

/// One entry of `File.hashes`; `algo` 1 is sha1, 2 is md5.
#[derive(Debug, Deserialize)]
struct CfHash {
    #[serde(default)]
    value: Option<String>,
    #[serde(default)]
    algo: Option<i64>,
}

fn sha1_of(hashes: &[CfHash]) -> Option<String> {
    hashes.iter().find(|hash| hash.algo == Some(1)).and_then(|hash| hash.value.clone())
}

#[derive(Debug, Deserialize)]
struct CfFileList {
    data: Vec<CfFile>,
}

#[derive(Debug, Deserialize)]
struct CfDownloadLink {
    data: String,
}

fn mod_to_search_result(mod_: CfMod, content: ContentType) -> SearchResult {
    SearchResult {
        id: mod_.id.to_string(),
        slug: Some(mod_.slug),
        name: mod_.name,
        author: mod_.authors.into_iter().next().map(|author| author.name),
        description: mod_.summary,
        downloads: mod_.download_count,
        icon_url: mod_.logo.and_then(|logo| logo.url.or(logo.thumbnail_url)),
        web_url: mod_.links.and_then(|links| links.website_url),
        updated: mod_.date_modified,
        content_type: content,
    }
}

fn file_to_content_version(project_id: &str, file: CfFile, content: ContentType) -> ContentVersion {
    ContentVersion {
        id: file.id.to_string(),
        project_id: Some(project_id.to_string()),
        name: file.display_name.clone(),
        version_number: file.display_name.unwrap_or(file.file_name.clone()),
        content_type: content,
        minecraft_versions: file.game_versions,
        loaders: loader_name(file.mod_loader),
        downloads: 0,
        date: file.file_date,
        changelog_url: None,
        files: vec![ContentFile {
            name: file.file_name,
            url: file.download_url.unwrap_or_default(),
            sha1: sha1_of(&file.hashes),
            size: Some(file.file_length),
            primary: true,
        }],
    }
}

impl sealed::Sealed for CurseForge {}
impl PlatformKind for CurseForge {
    const NAME: &'static str = "curseforge";
    const CONTENT_TYPES: &'static [ContentType] = &[ContentType::Mod, ContentType::Modpack, ContentType::ResourcePack, ContentType::World];
    const SUPPORTS_MODPACK_ARCHIVE: bool = true;

    async fn search(_http: &reqwest::Client, credential: Option<&str>, query: &SearchQuery) -> Result<SearchPage, PlatformError> {
        let Some(class_id) = class_id(query.content) else {
            return Ok(SearchPage::default());
        };
        let mut path = format!(
            "/mods/search?gameId={MINECRAFT_GAME_ID}&classId={class_id}&index={}&pageSize={}&sortField={}&sortOrder=desc",
            query.offset,
            query.limit.clamp(1, MAX_PAGE),
            sort_field(query.sort),
        );
        if !query.text.is_empty() {
            path = with_query(&path, "searchFilter", &query.text);
        }
        if let Some(category) = &query.category {
            path = with_query(&path, "categoryId", category);
        }
        if let Some((_, id)) = query
            .loader
            .as_deref()
            .and_then(|loader| LOADERS.iter().find(|(name, _)| *name == loader))
        {
            path = format!("{path}&modLoaderType={id}");
        }
        if let Some(version) = &query.game_version {
            path = with_query(&path, "gameVersion", version);
        }
        let (url, client) = request(credential, &path)?;
        let response = get_json(&client, &url).await?;
        let list: CfModList = serde_json::from_value(response).map_err(|source| PlatformError::Json { url: url.clone(), source })?;
        let total = list.pagination.map_or(list.data.len() as u64, |pagination| pagination.total_count);
        Ok(SearchPage {
            hits: list.data.into_iter().map(|cf_mod| mod_to_search_result(cf_mod, query.content)).collect(),
            total,
        })
    }

    async fn facets(_http: &reqwest::Client, credential: Option<&str>, content: ContentType) -> Result<Facets, PlatformError> {
        let Some(class_id) = class_id(content) else {
            return Ok(Facets::default());
        };
        let (url, client) = request(credential, &format!("/categories?gameId={MINECRAFT_GAME_ID}&classId={class_id}"))?;
        let categories: CfCategoryList =
            serde_json::from_value(get_json(&client, &url).await?).map_err(|source| PlatformError::Json { url: url.clone(), source })?;
        let (url, client) = request(credential, "/minecraft/version")?;
        let versions: CfGameVersionList =
            serde_json::from_value(get_json(&client, &url).await?).map_err(|source| PlatformError::Json { url: url.clone(), source })?;
        Ok(Facets {
            categories: categories
                .data
                .into_iter()
                .map(|category| Facet {
                    id: category.id.to_string(),
                    name: category.name,
                })
                .collect(),
            loaders: LOADERS.iter().map(|(name, _)| (*name).to_string()).collect(),
            game_versions: versions.data.into_iter().map(|version| version.version_string).collect(),
        })
    }

    async fn details(_http: &reqwest::Client, credential: Option<&str>, id: &str, content: ContentType) -> Result<ContentDetails, PlatformError> {
        let (url, client) = request(credential, &format!("/mods/{id}"))?;
        let response = get_json(&client, &url).await?;
        let envelope: CfModEnvelope = serde_json::from_value(response).map_err(|source| PlatformError::Json { url: url.clone(), source })?;

        // Long-form descriptions come from a separate endpoint; a failure there
        // is not worth failing the whole call over.
        let description = match request(credential, &format!("/mods/{id}/description")) {
            Ok((url, client)) => get_json(&client, &url)
                .await
                .ok()
                .and_then(|response| serde_json::from_value::<CfDescription>(response).ok().map(|envelope| envelope.data)),
            Err(_) => None,
        };

        Ok(ContentDetails {
            id: envelope.data.id.to_string(),
            slug: Some(envelope.data.slug),
            name: envelope.data.name,
            author: envelope.data.authors.into_iter().next().map(|author| author.name),
            summary: envelope.data.summary,
            description,
            downloads: envelope.data.download_count,
            icon_url: envelope.data.logo.and_then(|logo| logo.url.or(logo.thumbnail_url)),
            web_url: envelope.data.links.and_then(|links| links.website_url),
            content_type: content,
        })
    }

    async fn versions(
        _http: &reqwest::Client,
        credential: Option<&str>,
        id: &str,
        content: ContentType,
    ) -> Result<Vec<ContentVersion>, PlatformError> {
        let (url, client) = request(credential, &format!("/mods/{id}/files"))?;
        let response = get_json(&client, &url).await?;
        let list: CfFileList = serde_json::from_value(response).map_err(|source| PlatformError::Json { url: url.clone(), source })?;
        Ok(list.data.into_iter().map(|file| file_to_content_version(id, file, content)).collect())
    }

    async fn download(
        _http: &reqwest::Client,
        credential: Option<&str>,
        version: &ContentVersion,
        into: &Path,
        progress: Option<&Sender<DownloadProgress>>,
    ) -> Result<Vec<PathBuf>, PlatformError> {
        let mut files = Vec::new();
        for file in &version.files {
            let url = if file.url.is_empty() {
                // Some files hide their download url; the download-link
                // endpoint resolves the CDN url. That needs the project id,
                // which `versions()` always attaches for CurseForge.
                let Some(project_id) = version.project_id.as_deref() else {
                    return Err(PlatformError::Download {
                        url: file.name.clone(),
                        source: "the download url is hidden and the version carries no project id to resolve it".into(),
                    });
                };
                let (url, client) = request(credential, &format!("/mods/{project_id}/files/{}/download-link", version.id))?;
                let response = get_json(&client, &url).await?;
                let link: CfDownloadLink = serde_json::from_value(response).map_err(|source| PlatformError::Json { url: url.clone(), source })?;
                link.data
            } else {
                file.url.clone()
            };
            files.push((url, into.join(&file.name)));
        }
        download_files(files, progress).await
    }

    fn parse_modpack(archive: &Path) -> Result<crate::meta::Modpack, PlatformError> {
        crate::modpack::parse(archive)
    }
}

/// The CDN url of one file of a project. Modpack files name their mods by project and file id, and
/// each id resolves here when the modpack is installed. A file whose author disabled distribution has
/// no url, and the call fails. Reached through `Platform::<CurseForge>::file_url`.
pub(crate) async fn file_url(credential: Option<&str>, project_id: &str, file_id: &str) -> Result<String, PlatformError> {
    let (url, client) = request(credential, &format!("/mods/{project_id}/files/{file_id}/download-link"))?;
    let response = get_json(&client, &url).await?;
    let link: CfDownloadLink = serde_json::from_value(response).map_err(|source| PlatformError::Json { url: url.clone(), source })?;
    Ok(link.data)
}

#[cfg(test)]
#[path = "../tests/curseforge.rs"]
mod tests;
