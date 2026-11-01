//! Modrinth — delegated to ferinth (the gorilla-devs Modrinth API wrapper).
//!
//! Search and the tag lists are hand-rolled: ferinth serializes its facets
//! through `Debug`, which Modrinth answers with zero hits.

use std::path::{Path, PathBuf};
use std::sync::LazyLock;

use ferinth::Ferinth;
use ferinth::structures::project::{GalleryItem, SideType};
use ferinth::structures::user::TeamMember;
use ferinth::structures::version::{Dependency, DependencyType, VersionType};
use serde::Deserialize;
use simple_download_utility::DownloadProgress;
use tokio::sync::mpsc::Sender;

use crate::PlatformKind;
use crate::error::PlatformError;
use crate::http::{download_files, get_json, with_query};
use crate::meta::{
    BundledProject, Channel, ContentDetails, ContentFile, ContentType, ContentVersion, Creator, DependencyKind, DescriptionFormat, Facet, Facets,
    GalleryImage, ProjectInfo, ProjectLinks, SearchPage, SearchQuery, SearchResult, SearchSort, Side, VersionDependency, banner_from, display_name,
};
use crate::sealed;

/// Marker for Modrinth (https://modrinth.com).
pub struct Modrinth;

const SEARCH_BASE: &str = "https://api.modrinth.com/v2/search";
const TAG_BASE: &str = "https://api.modrinth.com/v2/tag";
const PROJECTS_BASE: &str = "https://api.modrinth.com/v2/projects";
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
    /// The gallery's thumbnails, in the order the project lists them.
    #[serde(default)]
    gallery: Vec<String>,
    #[serde(default)]
    featured_gallery: Option<String>,
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
    let banner_url = banner_from(hit.featured_gallery, hit.gallery);
    SearchResult {
        web_url: Some(web_url(&hit.project_id)),
        id: hit.project_id,
        slug: hit.slug,
        name: hit.title,
        author: Some(hit.author),
        description: hit.description,
        downloads: hit.downloads,
        icon_url: hit.icon_url,
        banner_url,
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

/// The sides a project runs on: each one it is required or optional on.
fn environments(client: SideType, server: SideType) -> Vec<Side> {
    [(client, Side::Client), (server, Side::Server)]
        .into_iter()
        .filter(|(support, _)| matches!(support, SideType::Required | SideType::Optional))
        .map(|(_, side)| side)
        .collect()
}

/// The licence's name, or its id when it has no name. `LicenseRef-…` licences have none.
fn license_name(name: String, id: String) -> Option<String> {
    let name = if name.trim().is_empty() { id } else { name };
    Some(name).filter(|name| !name.is_empty())
}

/// The team's creators with the owner first. The sort is stable, so the rest keep the team's order.
fn owner_first(mut creators: Vec<Creator>) -> Vec<Creator> {
    creators.sort_by_key(|creator| !creator.role.as_deref().is_some_and(|role| role.eq_ignore_ascii_case("owner")));
    creators
}

fn creator(member: TeamMember) -> Creator {
    Creator {
        name: member.user.name.filter(|name| !name.trim().is_empty()).unwrap_or(member.user.username),
        role: Some(member.role).filter(|role| !role.is_empty()),
        avatar_url: member.user.avatar_url.map(|url| url.to_string()),
    }
}

/// The accepted members of a project's team. A team that cannot be read has none, and the project still loads.
async fn team_creators(team: &str) -> Vec<Creator> {
    match API.team_list_members(team).await {
        Ok(members) => owner_first(members.into_iter().filter(|member| member.accepted).map(creator).collect()),
        Err(error) => {
            tracing::warn!("reading the team of a Modrinth project failed: {error}");
            Vec::new()
        }
    }
}

fn gallery_image(item: GalleryItem) -> GalleryImage {
    GalleryImage {
        thumbnail_url: item.url.to_string(),
        full_url: item.raw_url.to_string(),
        title: item.title,
        description: item.description,
        featured: item.featured,
    }
}

fn channel(version_type: VersionType) -> Channel {
    match version_type {
        VersionType::Release => Channel::Release,
        VersionType::Beta => Channel::Beta,
        VersionType::Alpha => Channel::Alpha,
    }
}

/// A dependency Modrinth names. The kinds this crate does not model are left out.
fn dependency(dependency: Dependency) -> Option<VersionDependency> {
    let kind = match dependency.dependency_type {
        DependencyType::Required => DependencyKind::Required,
        DependencyType::Optional => DependencyKind::Optional,
        DependencyType::Incompatible => DependencyKind::Incompatible,
        DependencyType::Embedded => DependencyKind::Embedded,
        DependencyType::Other => return None,
    };
    Some(VersionDependency {
        kind,
        project_id: dependency.project_id,
        version_id: dependency.version_id,
    })
}

/// The projects a modpack release bundles, for its contents list. The projects' names and icons come from one
/// batched call, and the bundled releases' numbers from another. A release with no bundled projects costs no call.
pub(crate) async fn bundled(version: &ContentVersion) -> Result<Vec<BundledProject>, PlatformError> {
    let embedded: Vec<&VersionDependency> = version
        .dependencies
        .iter()
        .filter(|dependency| dependency.kind == DependencyKind::Embedded && dependency.project_id.is_some())
        .collect();
    if embedded.is_empty() {
        return Ok(Vec::new());
    }
    let project_ids: Vec<&str> = embedded.iter().filter_map(|dependency| dependency.project_id.as_deref()).collect();
    let projects = API
        .project_get_multiple(&project_ids)
        .await
        .map_err(|source| ferinth_error(PROJECTS_BASE, source))?;
    let version_ids: Vec<&str> = embedded.iter().filter_map(|dependency| dependency.version_id.as_deref()).collect();
    let releases = if version_ids.is_empty() {
        Vec::new()
    } else {
        API.version_get_multiple(&version_ids)
            .await
            .map_err(|source| ferinth_error(PROJECTS_BASE, source))?
    };
    Ok(embedded
        .into_iter()
        .filter_map(|dependency| {
            let project_id = dependency.project_id.clone()?;
            let project = projects.iter().find(|project| project.id == project_id);
            let version_number = dependency
                .version_id
                .as_deref()
                .and_then(|id| releases.iter().find(|release| release.id == id))
                .map(|release| release.version_number.clone());
            Some(BundledProject {
                name: project.map_or_else(|| project_id.clone(), |project| project.title.clone()),
                icon_url: project.and_then(|project| project.icon_url.as_ref().map(ToString::to_string)),
                version_id: dependency.version_id.clone(),
                version_number,
                project_id,
            })
        })
        .collect())
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
        let creators = team_creators(&project.team).await;
        let author = creators.first().map(|creator| creator.name.clone());
        Ok(ContentDetails {
            id: project.id,
            slug: Some(project.slug),
            name: project.title,
            author,
            summary: project.description,
            description: Some(project.body),
            description_format: DescriptionFormat::Markdown,
            downloads: project.downloads as u64,
            icon_url: project.icon_url.map(|url| url.to_string()),
            web_url: Some(url),
            content_type: content,
            info: ProjectInfo {
                gallery: project.gallery.into_iter().map(gallery_image).collect(),
                license: license_name(project.license.name, project.license.id),
                published: Some(project.published.to_rfc3339()),
                updated: Some(project.updated.to_rfc3339()),
                followers: Some(project.followers as u64),
                categories: project.categories.into_iter().chain(project.additional_categories).collect(),
                loaders: project.loaders,
                game_versions: project.game_versions,
                environments: environments(project.client_side, project.server_side),
                links: ProjectLinks {
                    issues: project.issues_url.map(|url| url.to_string()),
                    source: project.source_url.map(|url| url.to_string()),
                    wiki: project.wiki_url.map(|url| url.to_string()),
                    discord: project.discord_url.map(|url| url.to_string()),
                    donate: project.donation_urls.into_iter().map(|link| link.url.to_string()).collect(),
                },
                creators,
            },
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
                changelog: version.changelog,
                channel: Some(channel(version.version_type)),
                dependencies: version.dependencies.into_iter().filter_map(dependency).collect(),
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
