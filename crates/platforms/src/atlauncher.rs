//! ATLauncher — hand-rolled binding for `api.atlauncher.com/v1` and the pack CDN.
//!
//! Packs are named by their `safeName` (non-alphanumerics removed, case
//! sensitive). The v1 API lists packs and their versions, and search filters the
//! `/packs/simple` list locally. A version's mods are not in the API: they are in
//! its `Configs.json` on the CDN, which names the pack's loader, lists each mod
//! with where it comes from, and points at a `Configs.zip` of config files.

use std::path::{Path, PathBuf};

use futures_util::future::join_all;
use serde::Deserialize;
use simple_download_utility::DownloadProgress;
use tokio::sync::mpsc::Sender;

use crate::PlatformKind;
use crate::error::PlatformError;
use crate::http::{HYDRATION_CHUNK, get_json};
use crate::meta::{ContentDetails, ContentType, ContentVersion, Facets, Modpack, ModpackArchive, ModpackFile, SearchPage, SearchQuery, SearchResult};
use crate::sealed;

/// Marker for ATLauncher (https://atlauncher.com).
pub struct AtLauncher;

const BASE: &str = "https://api.atlauncher.com/v1";
const WEB_BASE: &str = "https://atlauncher.com/pack/";
const ICON_BASE: &str = "https://cdn.atlcdn.net/images/packs/";
/// Where a pack's files are served. Each mod's and the pack JSON's path is relative to it.
const CDN_BASE: &str = "https://download.nodecdn.net/containers/atl";

/// The ATLauncher API envelope: `{ error, code, message, data }`.
#[derive(Debug, Deserialize)]
struct Envelope<T> {
    data: Option<T>,
}

/// One row of `/packs/simple`.
#[derive(Debug, Deserialize)]
struct SimplePack {
    name: String,
    #[serde(rename = "safeName")]
    safe_name: String,
    #[serde(rename = "type", default)]
    pack_type: Option<String>,
}

/// A pack's details, from `/pack/{safeName}`.
#[derive(Debug, Deserialize)]
struct Pack {
    name: String,
    #[serde(rename = "safeName", default)]
    safe_name: Option<String>,
    #[serde(default)]
    description: Option<String>,
    #[serde(default)]
    versions: Vec<PackVersion>,
}

#[derive(Debug, Deserialize)]
struct PackVersion {
    version: String,
    #[serde(rename = "minecraft")]
    minecraft_version: String,
    #[serde(default)]
    published: Option<u64>,
}

/// A version's `Configs.json`: its Minecraft version, loader, mods and config archive.
#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
struct PackConfigs {
    minecraft: String,
    #[serde(default)]
    loader: Option<ConfigsLoader>,
    #[serde(default)]
    mods: Vec<ConfigsMod>,
    #[serde(default)]
    configs: Option<ConfigsArchive>,
    #[serde(default)]
    no_configs: bool,
}

/// The loader a pack names: its type (`fabric`, `forge`, ...) and its metadata, which carries the loader's version.
#[derive(Debug, Deserialize)]
struct ConfigsLoader {
    #[serde(rename = "type")]
    kind: String,
    #[serde(default)]
    metadata: Option<serde_json::Value>,
}

/// One mod of a pack version. `download` says where it comes from: `server` (the CDN), `direct`
/// (its own url), or `browser` (its author forbids the launcher to fetch it).
#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
struct ConfigsMod {
    name: String,
    #[serde(default)]
    url: Option<String>,
    #[serde(default)]
    file: Option<String>,
    #[serde(default)]
    download: String,
    #[serde(default)]
    filesize: Option<u64>,
    #[serde(rename = "type", default)]
    kind: String,
    #[serde(default = "yes")]
    client: bool,
    #[serde(default)]
    optional: bool,
    #[serde(default)]
    selected: bool,
    #[serde(default)]
    extract_to: Option<String>,
    #[serde(default)]
    extract_folder: Option<String>,
}

/// The config archive of a pack version.
#[derive(Debug, Deserialize)]
struct ConfigsArchive {
    #[serde(default)]
    sha1: Option<String>,
    #[serde(default)]
    filesize: Option<u64>,
}

fn yes() -> bool {
    true
}

fn web_url(safe_name: &str) -> String {
    format!("{WEB_BASE}{safe_name}")
}

/// The pack's icon, which ATLauncher serves by lowercased safe name.
fn icon_url(safe_name: &str) -> String {
    format!("{ICON_BASE}{}.png", safe_name.to_lowercase())
}

/// Where a version's `Configs.json` is served.
fn configs_url(safe_name: &str, version: &str) -> String {
    format!("{CDN_BASE}/packs/{safe_name}/versions/{version}/Configs.json")
}

/// Where a version's `Configs.zip` is served.
fn configs_zip_url(safe_name: &str, version: &str) -> String {
    format!("{CDN_BASE}/packs/{safe_name}/versions/{version}/Configs.zip")
}

/// Fetches one pack's details by its safe name.
async fn fetch_pack(http: &reqwest::Client, safe_name: &str) -> Result<Pack, PlatformError> {
    let url = format!("{BASE}/pack/{safe_name}");
    let response = get_json(http, &url).await?;
    let envelope: Envelope<Pack> = serde_json::from_value(response).map_err(|source| PlatformError::Json { url: url.clone(), source })?;
    envelope.data.ok_or(PlatformError::UnexpectedResponse { url, status: 200 })
}

/// Fetches the details of each listed pack, a few at a time, and keeps the ones that load. The
/// results keep the list's order.
async fn hydrate(http: &reqwest::Client, packs: &[SimplePack]) -> Vec<SearchResult> {
    let mut hits = Vec::with_capacity(packs.len());
    for chunk in packs.chunks(HYDRATION_CHUNK) {
        let mut calls = Vec::with_capacity(chunk.len());
        for pack in chunk {
            calls.push(fetch_pack(http, &pack.safe_name));
        }
        for (pack, loaded) in chunk.iter().zip(join_all(calls).await) {
            if let Ok(detail) = loaded {
                hits.push(SearchResult {
                    id: pack.safe_name.clone(),
                    slug: Some(pack.safe_name.clone()),
                    name: detail.name,
                    author: None,
                    description: detail.description.unwrap_or_default(),
                    downloads: 0,
                    icon_url: Some(icon_url(&pack.safe_name)),
                    web_url: Some(web_url(&pack.safe_name)),
                    updated: None,
                    content_type: ContentType::Modpack,
                });
            }
        }
    }
    hits
}

/// The loader a pack names, as `lodestone_loaders` names it.
fn loader_name(kind: &str) -> Option<&'static str> {
    ["fabric", "forge", "neoforge", "quilt"].into_iter().find(|known| *known == kind)
}

/// The folder a mod of this type goes in, or `None` for a type the launcher does not install.
fn folder_for(kind: &str) -> Option<&'static str> {
    match kind {
        "mods" | "jar" | "dependency" | "depandency" => Some("mods"),
        "coremods" => Some("coremods"),
        "resourcepack" | "texturepack" => Some("resourcepacks"),
        "shaderpack" => Some("shaderpacks"),
        _ => None,
    }
}

/// Whether a mod of this type ships as an archive to unpack, rather than a file to place.
fn is_archive(kind: &str) -> bool {
    matches!(kind, "extract" | "texturepackextract" | "resourcepackextract")
}

/// The folder an archive unpacks into, from ATLauncher's `extractTo` names.
fn extract_target(extract_to: Option<&str>, kind: &str) -> String {
    match extract_to {
        Some("mods") => "mods".to_string(),
        Some("coremods") => "coremods".to_string(),
        Some("root") | None if kind.starts_with("resourcepack") || kind.starts_with("texturepack") => "resourcepacks".to_string(),
        _ => String::new(),
    }
}

/// The folder inside an archive that unpacks, without slashes. A root folder means the whole archive.
fn extract_prefix(folder: Option<&str>) -> String {
    folder.unwrap_or_default().trim_matches('/').to_string()
}

/// The address a mod is downloaded from, by where the pack says it comes from.
fn download_url(name: &str, download: &str, url: Option<&str>) -> Result<String, PlatformError> {
    match (download, url) {
        ("server", Some(path)) => Ok(format!("{CDN_BASE}/{}", path.trim_start_matches('/'))),
        ("direct", Some(url)) => Ok(url.to_string()),
        _ => Err(PlatformError::Download {
            url: name.to_string(),
            source: format!("{name} must be downloaded by hand from its website").into(),
        }),
    }
}

/// The installable modpack of a pack version, from its `Configs.json`. A mod the client does not run,
/// and an optional mod nobody selected, are left out. A mod the launcher cannot place is an error,
/// because installing without it would leave a broken instance.
fn modpack_from(name: String, safe_name: &str, version: &ContentVersion, configs: PackConfigs) -> Result<Modpack, PlatformError> {
    let loader = match &configs.loader {
        Some(loader) => Some(loader_name(&loader.kind).ok_or_else(|| PlatformError::UnsupportedPlatform {
            platform: AtLauncher::NAME,
            action: "installing a pack with this loader",
        })?),
        None => None,
    };
    let loader_version = configs
        .loader
        .as_ref()
        .and_then(|loader| loader.metadata.as_ref())
        .and_then(|metadata| metadata.get("loader"))
        .and_then(serde_json::Value::as_str)
        .map(|version| match loader {
            // Forge's installer takes `minecraft-forge`, while the other loaders take their own version.
            Some("forge") => format!("{}-{version}", configs.minecraft),
            _ => version.to_string(),
        });

    let mut files = Vec::new();
    let mut archives = Vec::new();
    for entry in configs.mods {
        if !entry.client || (entry.optional && !entry.selected) {
            continue;
        }
        // The loader's installer provides Forge itself, so a Forge jar in the list is redundant.
        if entry.kind == "forge" && loader == Some("forge") {
            continue;
        }
        let url = download_url(&entry.name, &entry.download, entry.url.as_deref())?;
        let file = entry
            .file
            .clone()
            .or_else(|| url.rsplit('/').next().map(str::to_string))
            .unwrap_or_else(|| entry.name.clone());
        if is_archive(&entry.kind) {
            archives.push(ModpackArchive {
                name: file,
                url,
                sha1: None,
                size: entry.filesize,
                target: extract_target(entry.extract_to.as_deref(), &entry.kind),
                prefix: extract_prefix(entry.extract_folder.as_deref()),
            });
            continue;
        }
        let Some(folder) = folder_for(&entry.kind) else {
            return Err(PlatformError::UnsupportedPlatform {
                platform: AtLauncher::NAME,
                action: "installing a mod of this type",
            });
        };
        files.push(ModpackFile {
            path: format!("{folder}/{file}"),
            url,
            sha1: None,
            size: entry.filesize,
            project_id: None,
            file_id: None,
        });
    }

    if let Some(configs_archive) = configs.configs.as_ref()
        && !configs.no_configs
    {
        archives.push(ModpackArchive {
            name: "Configs.zip".to_string(),
            url: configs_zip_url(safe_name, &version.id),
            sha1: configs_archive.sha1.clone(),
            size: configs_archive.filesize,
            target: String::new(),
            prefix: String::new(),
        });
    }

    Ok(Modpack {
        name,
        version: Some(version.version_number.clone()),
        minecraft: configs.minecraft,
        loader,
        loader_version,
        files,
        overrides: None,
        overrides_skip: Vec::new(),
        archives,
    })
}

/// The installable modpack of one pack version: its `Configs.json` from the CDN, turned into the mods
/// and archives the install places. Reached through `Platform::<AtLauncher>::modpack`.
pub(crate) async fn modpack(http: &reqwest::Client, version: &ContentVersion) -> Result<Modpack, PlatformError> {
    let Some(safe_name) = version.project_id.as_deref() else {
        return Err(PlatformError::Download {
            url: BASE.to_string(),
            source: "the version carries no pack name to fetch its configs by".into(),
        });
    };
    let pack = fetch_pack(http, safe_name).await?;
    let url = configs_url(safe_name, &version.id);
    let response = get_json(http, &url).await?;
    let configs: PackConfigs = serde_json::from_value(response).map_err(|source| PlatformError::Json { url: url.clone(), source })?;
    modpack_from(pack.name, safe_name, version, configs)
}

impl sealed::Sealed for AtLauncher {}
impl PlatformKind for AtLauncher {
    const NAME: &'static str = "atlauncher";
    const CONTENT_TYPES: &'static [ContentType] = &[ContentType::Modpack];
    const SUPPORTS_MODPACK_ARCHIVE: bool = false;

    async fn search(http: &reqwest::Client, _credential: Option<&str>, query: &SearchQuery) -> Result<SearchPage, PlatformError> {
        let url = format!("{BASE}/packs/simple");
        let response = get_json(http, &url).await?;
        let envelope: Envelope<Vec<SimplePack>> =
            serde_json::from_value(response).map_err(|source| PlatformError::Json { url: url.clone(), source })?;
        let needle = query.text.trim().to_lowercase();
        let matching: Vec<SimplePack> = envelope
            .data
            .unwrap_or_default()
            .into_iter()
            .filter(|pack| pack.pack_type.as_deref().is_none_or(|kind| kind == "public"))
            .filter(|pack| needle.is_empty() || pack.name.to_lowercase().contains(&needle))
            .collect();
        let total = matching.len() as u64;
        let page: Vec<SimplePack> = matching.into_iter().skip(query.offset as usize).take(query.limit as usize).collect();
        Ok(SearchPage {
            hits: hydrate(http, &page).await,
            total,
        })
    }

    async fn facets(_http: &reqwest::Client, _credential: Option<&str>, _content: ContentType) -> Result<Facets, PlatformError> {
        Ok(Facets::default())
    }

    async fn details(http: &reqwest::Client, _credential: Option<&str>, id: &str, _content: ContentType) -> Result<ContentDetails, PlatformError> {
        let pack = fetch_pack(http, id).await?;
        Ok(ContentDetails {
            id: id.to_string(),
            slug: pack.safe_name.or_else(|| Some(id.to_string())),
            name: pack.name,
            author: None,
            summary: pack.description.clone().unwrap_or_default(),
            description: pack.description,
            downloads: 0,
            icon_url: Some(icon_url(id)),
            web_url: Some(web_url(id)),
            content_type: ContentType::Modpack,
        })
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
            .map(|version| ContentVersion {
                id: version.version.clone(),
                project_id: Some(id.to_string()),
                name: Some(version.version.clone()),
                version_number: version.version,
                content_type: ContentType::Modpack,
                minecraft_versions: vec![version.minecraft_version],
                loaders: Vec::new(),
                downloads: 0,
                date: version.published.map(|published| published.to_string()),
                changelog_url: None,
                // A pack's files come from `Configs.json`, which `Platform::modpack` reads.
                files: Vec::new(),
            })
            .collect())
    }

    async fn download(
        _http: &reqwest::Client,
        _credential: Option<&str>,
        _version: &ContentVersion,
        _into: &Path,
        _progress: Option<&Sender<DownloadProgress>>,
    ) -> Result<Vec<PathBuf>, PlatformError> {
        Err(PlatformError::UnsupportedPlatform {
            platform: Self::NAME,
            action: "a single-file download (a pack installs from its Configs.json)",
        })
    }

    fn parse_modpack(_archive: &Path) -> Result<Modpack, PlatformError> {
        Err(PlatformError::UnsupportedPlatform {
            platform: Self::NAME,
            action: "modpack archive parsing",
        })
    }
}

#[cfg(test)]
#[path = "../tests/atlauncher.rs"]
mod tests;
