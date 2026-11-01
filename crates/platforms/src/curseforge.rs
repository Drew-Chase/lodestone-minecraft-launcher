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
use crate::meta::{
    Channel, ContentDetails, ContentFile, ContentType, ContentVersion, Creator, DependencyKind, DescriptionFormat, Facet, Facets, GalleryImage,
    ProjectInfo, ProjectLinks, SearchPage, SearchQuery, SearchResult, SearchSort, VersionDependency, banner_from,
};
use crate::sealed;

/// Marker for CurseForge (https://www.curseforge.com). Requires an API key —
/// see [`Platform::<CurseForge>::with_api_key`](crate::Platform::with_api_key).
pub struct CurseForge;

const BASE: &str = "https://api.curseforge.com/v1";
/// CurseForge's game id for Minecraft.
const MINECRAFT_GAME_ID: u32 = 432;
/// The largest page `/mods/search` returns.
const MAX_PAGE: u32 = 50;
/// The largest page `/mods/{id}/files` returns.
const FILE_PAGE: u32 = 50;
/// The most file pages one listing reads, which is 2,000 files.
const MAX_FILE_PAGES: usize = 40;

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
    #[serde(rename = "dateCreated", default)]
    date_created: Option<String>,
    #[serde(rename = "dateModified", default)]
    date_modified: Option<String>,
    #[serde(default)]
    authors: Vec<CfAuthor>,
    #[serde(default)]
    logo: Option<CfLogo>,
    #[serde(default)]
    links: Option<CfLinks>,
    #[serde(default)]
    categories: Vec<CfCategory>,
    #[serde(default)]
    screenshots: Vec<CfScreenshot>,
}

#[derive(Debug, Deserialize)]
struct CfAuthor {
    name: String,
    #[serde(rename = "avatarUrl", default)]
    avatar_url: Option<String>,
}

/// One picture of a project's gallery. CurseForge names the small copy `thumbnailUrl`.
#[derive(Debug, Deserialize)]
struct CfScreenshot {
    #[serde(default)]
    title: Option<String>,
    #[serde(default)]
    description: Option<String>,
    #[serde(rename = "thumbnailUrl", default)]
    thumbnail_url: Option<String>,
    #[serde(default)]
    url: Option<String>,
}

#[derive(Debug, Deserialize)]
struct CfLogo {
    #[serde(rename = "thumbnailUrl", default)]
    thumbnail_url: Option<String>,
    #[serde(default)]
    url: Option<String>,
}

#[derive(Debug, Default, Deserialize)]
struct CfLinks {
    #[serde(rename = "websiteUrl", default)]
    website_url: Option<String>,
    #[serde(rename = "wikiUrl", default)]
    wiki_url: Option<String>,
    #[serde(rename = "issuesUrl", default)]
    issues_url: Option<String>,
    #[serde(rename = "sourceUrl", default)]
    source_url: Option<String>,
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
    /// `1` is a release, `2` a beta, `3` an alpha.
    #[serde(rename = "releaseType", default)]
    release_type: Option<i64>,
    #[serde(default)]
    dependencies: Vec<CfDependency>,
}

/// One project a file relates to, with CurseForge's `relationType`.
#[derive(Debug, Deserialize)]
struct CfDependency {
    #[serde(rename = "modId")]
    mod_id: i64,
    #[serde(rename = "relationType")]
    relation_type: i64,
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
    #[serde(default)]
    pagination: Option<CfPagination>,
}

/// One file, as `/mods/{id}/files/{fileId}` answers it.
#[derive(Debug, Deserialize)]
struct CfFileEnvelope {
    data: CfFile,
}

#[derive(Debug, Deserialize)]
struct CfDownloadLink {
    data: String,
}

/// The banner a CurseForge project shows: its first screenshot. CurseForge marks no featured one.
fn banner_of(screenshots: &[CfScreenshot]) -> Option<String> {
    banner_from(
        None,
        screenshots
            .iter()
            .filter_map(|shot| shot.thumbnail_url.clone().or_else(|| shot.url.clone())),
    )
}

/// The screenshots as a gallery. A screenshot with no picture at all is left out.
fn gallery_of(screenshots: Vec<CfScreenshot>) -> Vec<GalleryImage> {
    screenshots
        .into_iter()
        .filter_map(|shot| {
            let full_url = shot.url.clone().or_else(|| shot.thumbnail_url.clone())?;
            Some(GalleryImage {
                thumbnail_url: shot.thumbnail_url.unwrap_or_else(|| full_url.clone()),
                full_url,
                title: shot.title.filter(|title| !title.is_empty()),
                description: shot.description.filter(|description| !description.is_empty()),
                featured: false,
            })
        })
        .collect()
}

/// A file's release channel, from CurseForge's `releaseType`.
fn channel_of(release_type: Option<i64>) -> Option<Channel> {
    match release_type {
        Some(1) => Some(Channel::Release),
        Some(2) => Some(Channel::Beta),
        Some(3) => Some(Channel::Alpha),
        _ => None,
    }
}

/// How a file relates to another project, from CurseForge's `relationType`. The relations this crate does not model
/// (tools, and projects a modpack includes) are left out.
fn dependency_kind(relation_type: i64) -> Option<DependencyKind> {
    match relation_type {
        1 => Some(DependencyKind::Embedded),
        2 => Some(DependencyKind::Optional),
        3 => Some(DependencyKind::Required),
        5 => Some(DependencyKind::Incompatible),
        _ => None,
    }
}

fn mod_to_search_result(mod_: CfMod, content: ContentType) -> SearchResult {
    let banner_url = banner_of(&mod_.screenshots);
    SearchResult {
        id: mod_.id.to_string(),
        slug: Some(mod_.slug),
        name: mod_.name,
        author: mod_.authors.into_iter().next().map(|author| author.name),
        description: mod_.summary,
        downloads: mod_.download_count,
        icon_url: mod_.logo.and_then(|logo| logo.url.or(logo.thumbnail_url)),
        banner_url,
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
        changelog: None,
        channel: channel_of(file.release_type),
        dependencies: file
            .dependencies
            .iter()
            .filter_map(|dependency| {
                Some(VersionDependency {
                    kind: dependency_kind(dependency.relation_type)?,
                    project_id: Some(dependency.mod_id.to_string()),
                    version_id: None,
                })
            })
            .collect(),
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
        let data = serde_json::from_value::<CfModEnvelope>(response)
            .map_err(|source| PlatformError::Json { url: url.clone(), source })?
            .data;

        // Long-form descriptions come from a separate endpoint; a failure there
        // is not worth failing the whole call over.
        let description = match request(credential, &format!("/mods/{id}/description")) {
            Ok((url, client)) => get_json(&client, &url)
                .await
                .ok()
                .and_then(|response| serde_json::from_value::<CfDescription>(response).ok().map(|envelope| envelope.data)),
            Err(_) => None,
        };

        let CfLinks {
            website_url,
            wiki_url,
            issues_url,
            source_url,
        } = data.links.unwrap_or_default();
        let creators: Vec<Creator> = data
            .authors
            .into_iter()
            .map(|author| Creator {
                name: author.name,
                role: None,
                avatar_url: author.avatar_url,
            })
            .collect();
        let author = creators.first().map(|creator| creator.name.clone());
        Ok(ContentDetails {
            id: data.id.to_string(),
            slug: Some(data.slug),
            name: data.name,
            author,
            summary: data.summary,
            description,
            description_format: DescriptionFormat::Html,
            downloads: data.download_count,
            icon_url: data.logo.and_then(|logo| logo.url.or(logo.thumbnail_url)),
            web_url: website_url,
            content_type: content,
            info: ProjectInfo {
                gallery: gallery_of(data.screenshots),
                license: None,
                published: data.date_created,
                updated: data.date_modified,
                followers: None,
                categories: data.categories.into_iter().map(|category| category.name).collect(),
                loaders: Vec::new(),
                game_versions: Vec::new(),
                environments: Vec::new(),
                links: ProjectLinks {
                    issues: issues_url,
                    source: source_url,
                    wiki: wiki_url,
                    discord: None,
                    donate: Vec::new(),
                },
                creators,
            },
        })
    }

    async fn versions(
        _http: &reqwest::Client,
        credential: Option<&str>,
        id: &str,
        content: ContentType,
    ) -> Result<Vec<ContentVersion>, PlatformError> {
        let mut files: Vec<CfFile> = Vec::new();
        for _ in 0..MAX_FILE_PAGES {
            let (url, client) = request(credential, &format!("/mods/{id}/files?index={}&pageSize={FILE_PAGE}", files.len()))?;
            let response = get_json(&client, &url).await?;
            let list: CfFileList = serde_json::from_value(response).map_err(|source| PlatformError::Json { url: url.clone(), source })?;
            let total = list.pagination.map_or(0, |pagination| pagination.total_count);
            let received = list.data.len();
            files.extend(list.data);
            if received == 0 || files.len() as u64 >= total {
                break;
            }
        }
        Ok(files.into_iter().map(|file| file_to_content_version(id, file, content)).collect())
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

/// The files of a project that run on a Minecraft version and a loader, newest first. CurseForge filters the files
/// itself, so the first page holds every file that fits. Reached through `Platform::<CurseForge>::fitting_files`.
pub(crate) async fn fitting_files(
    credential: Option<&str>,
    project_id: &str,
    game_version: &str,
    loader: &str,
) -> Result<Vec<ContentVersion>, PlatformError> {
    let mut path = with_query(
        &format!("/mods/{project_id}/files?index=0&pageSize={FILE_PAGE}"),
        "gameVersion",
        game_version,
    );
    if let Some((_, id)) = LOADERS.iter().find(|(name, _)| *name == loader) {
        path = format!("{path}&modLoaderType={id}");
    }
    let (url, client) = request(credential, &path)?;
    let response = get_json(&client, &url).await?;
    let list: CfFileList = serde_json::from_value(response).map_err(|source| PlatformError::Json { url: url.clone(), source })?;
    Ok(list
        .data
        .into_iter()
        .map(|file| file_to_content_version(project_id, file, ContentType::Mod))
        .collect())
}

/// One file of a project: its name, its download url, and its sha1 and size. A file whose listing hides its url
/// is resolved through the download-link endpoint. Reached through `Platform::<CurseForge>::file`.
pub(crate) async fn file(credential: Option<&str>, project_id: &str, file_id: &str) -> Result<ContentFile, PlatformError> {
    let (url, client) = request(credential, &format!("/mods/{project_id}/files/{file_id}"))?;
    let response = get_json(&client, &url).await?;
    let file = serde_json::from_value::<CfFileEnvelope>(response)
        .map_err(|source| PlatformError::Json { url: url.clone(), source })?
        .data;
    let download = match file.download_url.filter(|link| !link.is_empty()) {
        Some(link) => link,
        None => file_url(credential, project_id, file_id).await?,
    };
    Ok(ContentFile {
        name: file.file_name,
        url: download,
        sha1: sha1_of(&file.hashes),
        size: Some(file.file_length),
        primary: true,
    })
}

#[cfg(test)]
#[path = "../tests/curseforge.rs"]
mod tests;
