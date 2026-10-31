//! Technic — hand-rolled binding for `api.technicpack.net` and Solder servers.
//!
//! Every Technic call names the launcher with `build=`: without it the API
//! answers 401. A pack comes in one of two shapes. A Solder pack lists its builds
//! on a Solder server, and a build's manifest names each mod with its download.
//! Any other pack is a zip of the whole instance, which carries its loader in
//! `bin/version.json`.

use std::path::{Path, PathBuf};

use serde::Deserialize;
use simple_download_utility::DownloadProgress;
use tokio::sync::mpsc::Sender;

use crate::PlatformKind;
use crate::error::PlatformError;
use crate::http::{download_files, get_json, with_query};
use crate::meta::{ContentDetails, ContentFile, ContentType, ContentVersion, Facets, Modpack, ModpackFile, SearchPage, SearchQuery, SearchResult};
use crate::sealed;

/// Marker for Technic (https://www.technicpack.net).
pub struct Technic;

const BASE: &str = "https://api.technicpack.net";
/// The launcher name every Technic call sends. The API answers 401 without it.
const BUILD: &str = "lodestone";
const WEB_BASE: &str = "https://www.technicpack.net/modpack/";

/// Adds the launcher name to a Technic url.
fn build_url(url: &str) -> String {
    with_query(url, "build", BUILD)
}

/// A search or trending hit: a pack's slug, name and picture.
#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
struct TechnicHit {
    slug: String,
    name: String,
    #[serde(default)]
    url: Option<String>,
    #[serde(default)]
    icon_url: Option<String>,
}

/// The envelope of `/search` and `/trending`.
#[derive(Debug, Deserialize)]
struct TechnicHits {
    #[serde(default)]
    modpacks: Vec<TechnicHit>,
}

/// A pack's details, from `/modpack/{slug}`.
#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
struct TechnicModpack {
    name: String,
    #[serde(default)]
    display_name: Option<String>,
    #[serde(default)]
    user: Option<String>,
    #[serde(default)]
    description: Option<String>,
    #[serde(default)]
    installs: Option<u64>,
    #[serde(default)]
    minecraft: Option<String>,
    #[serde(default)]
    version: Option<String>,
    /// The zip of the whole instance, for a pack that is not on a Solder server.
    #[serde(default)]
    url: Option<String>,
    /// The Solder server the pack's builds live on, when it has one.
    #[serde(default)]
    solder: Option<String>,
    #[serde(default)]
    icon: Option<TechnicAsset>,
    #[serde(default)]
    platform_url: Option<String>,
}

#[derive(Debug, Deserialize)]
struct TechnicAsset {
    #[serde(default)]
    url: Option<String>,
}

/// A Solder server's view of a pack: the builds it has.
#[derive(Debug, Deserialize)]
struct SolderPack {
    #[serde(default)]
    builds: Vec<String>,
}

/// A Solder build's manifest: its Minecraft version, its Forge version, and its mods.
#[derive(Debug, Deserialize)]
struct SolderManifest {
    #[serde(default)]
    minecraft: Option<String>,
    #[serde(default)]
    forge: Option<String>,
    #[serde(default)]
    mods: Vec<SolderMod>,
}

/// One mod of a Solder build, served from its own url.
#[derive(Debug, Deserialize)]
struct SolderMod {
    url: String,
    #[serde(default)]
    filesize: Option<u64>,
}

fn hit_to_result(hit: TechnicHit) -> SearchResult {
    SearchResult {
        id: hit.slug.clone(),
        slug: Some(hit.slug),
        name: hit.name,
        author: None,
        description: String::new(),
        downloads: 0,
        icon_url: hit.icon_url,
        web_url: hit.url,
        updated: None,
        content_type: ContentType::Modpack,
    }
}

/// The last segment of a mod's url, which is the file's name.
fn file_name(url: &str) -> String {
    url.rsplit('/').next().unwrap_or(url).to_string()
}

/// Fetches a pack's details by its slug.
async fn fetch_pack(http: &reqwest::Client, slug: &str) -> Result<TechnicModpack, PlatformError> {
    let url = build_url(&format!("{BASE}/modpack/{slug}"));
    let response = get_json(http, &url).await?;
    serde_json::from_value(response).map_err(|source| PlatformError::Json { url, source })
}

/// The Forge version a Solder manifest names, in the `minecraft-forge` form the Forge installer takes.
/// A manifest that already writes it that way is left as it is. Without a Minecraft version there is
/// nothing to combine it with, so the Forge version stands alone.
fn combined_forge(minecraft: &str, forge: &str) -> String {
    if minecraft.is_empty() || forge.starts_with(&format!("{minecraft}-")) {
        forge.to_string()
    } else {
        format!("{minecraft}-{forge}")
    }
}

/// The details of a pack, from its record on the Technic API.
fn details_of(id: &str, pack: TechnicModpack) -> ContentDetails {
    ContentDetails {
        id: id.to_string(),
        slug: Some(id.to_string()),
        name: pack.display_name.clone().unwrap_or_else(|| pack.name.clone()),
        author: pack.user,
        summary: pack.description.clone().unwrap_or_default(),
        description: pack.description,
        downloads: pack.installs.unwrap_or(0),
        icon_url: pack.icon.and_then(|icon| icon.url),
        web_url: Some(pack.platform_url.unwrap_or_else(|| format!("{WEB_BASE}{id}"))),
        content_type: ContentType::Modpack,
    }
}

/// One version per Solder build, newest first: Solder lists its builds oldest first.
fn solder_versions(id: &str, minecraft: Option<&str>, builds: Vec<String>) -> Vec<ContentVersion> {
    let minecraft_versions: Vec<String> = minecraft.map(str::to_string).into_iter().collect();
    builds
        .into_iter()
        .rev()
        .map(|build| ContentVersion {
            id: build.clone(),
            project_id: Some(id.to_string()),
            name: Some(build.clone()),
            version_number: build,
            content_type: ContentType::Modpack,
            minecraft_versions: minecraft_versions.clone(),
            loaders: Vec::new(),
            downloads: 0,
            date: None,
            changelog_url: None,
            files: Vec::new(),
        })
        .collect()
}

/// The one version of a pack that is only a zip: the zip its pack page links to.
fn zip_versions(id: &str, pack: &TechnicModpack) -> Vec<ContentVersion> {
    let version = pack.version.clone().unwrap_or_else(|| "latest".to_string());
    let files = pack
        .url
        .as_ref()
        .map(|url| ContentFile {
            name: format!("{id}-{version}.zip"),
            url: url.clone(),
            sha1: None,
            size: None,
            primary: true,
        })
        .into_iter()
        .collect();
    vec![ContentVersion {
        id: version.clone(),
        project_id: Some(id.to_string()),
        name: Some(version.clone()),
        version_number: version,
        content_type: ContentType::Modpack,
        minecraft_versions: pack.minecraft.iter().cloned().collect(),
        loaders: Vec::new(),
        downloads: 0,
        date: None,
        changelog_url: None,
        files,
    }]
}

impl sealed::Sealed for Technic {}
impl PlatformKind for Technic {
    const NAME: &'static str = "technic";
    const CONTENT_TYPES: &'static [ContentType] = &[ContentType::Modpack];
    const SUPPORTS_MODPACK_ARCHIVE: bool = true;

    async fn search(http: &reqwest::Client, _credential: Option<&str>, query: &SearchQuery) -> Result<SearchPage, PlatformError> {
        let text = query.text.trim();
        let url = if text.is_empty() {
            build_url(&format!("{BASE}/trending"))
        } else {
            with_query(&build_url(&format!("{BASE}/search")), "q", text)
        };
        let response = get_json(http, &url).await?;
        let hits: TechnicHits = serde_json::from_value(response).map_err(|source| PlatformError::Json { url: url.clone(), source })?;
        let matching: Vec<SearchResult> = hits.modpacks.into_iter().map(hit_to_result).collect();
        let total = matching.len() as u64;
        let page = matching.into_iter().skip(query.offset as usize).take(query.limit as usize).collect();
        Ok(SearchPage { hits: page, total })
    }

    async fn facets(_http: &reqwest::Client, _credential: Option<&str>, _content: ContentType) -> Result<Facets, PlatformError> {
        Ok(Facets::default())
    }

    async fn details(http: &reqwest::Client, _credential: Option<&str>, id: &str, _content: ContentType) -> Result<ContentDetails, PlatformError> {
        Ok(details_of(id, fetch_pack(http, id).await?))
    }

    async fn versions(
        http: &reqwest::Client,
        _credential: Option<&str>,
        id: &str,
        _content: ContentType,
    ) -> Result<Vec<ContentVersion>, PlatformError> {
        let pack = fetch_pack(http, id).await?;
        let Some(solder) = pack.solder.as_deref() else {
            return Ok(zip_versions(id, &pack));
        };
        // Solder answers on its own paths, which take no launcher name.
        let url = format!("{}/modpack/{id}", solder.trim_end_matches('/'));
        let response = get_json(http, &url).await?;
        let solder_pack: SolderPack = serde_json::from_value(response).map_err(|source| PlatformError::Json { url, source })?;
        Ok(solder_versions(id, pack.minecraft.as_deref(), solder_pack.builds))
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

    fn parse_modpack(archive: &Path) -> Result<Modpack, PlatformError> {
        crate::modpack::technic::parse(archive)
    }
}

/// The installable modpack of a Solder build: the mods its manifest names, and the Minecraft and
/// Forge versions it targets. Reached through `Platform::<Technic>::modpack`.
pub(crate) async fn modpack(http: &reqwest::Client, version: &ContentVersion) -> Result<Modpack, PlatformError> {
    let Some(slug) = version.project_id.as_deref() else {
        return Err(PlatformError::Download {
            url: BASE.to_string(),
            source: "the version carries no pack slug to fetch its build by".into(),
        });
    };
    let pack = fetch_pack(http, slug).await?;
    let Some(solder) = pack.solder.as_deref() else {
        return Err(PlatformError::UnsupportedPlatform {
            platform: Technic::NAME,
            action: "installing a zip pack from its modpack zip",
        });
    };
    let url = format!("{}/modpack/{slug}/{}", solder.trim_end_matches('/'), version.id);
    let response = get_json(http, &url).await?;
    let manifest: SolderManifest = serde_json::from_value(response).map_err(|source| PlatformError::Json { url, source })?;
    let name = pack.display_name.clone().unwrap_or_else(|| pack.name.clone());
    solder_modpack(name, pack.minecraft, version, manifest)
}

/// The installable modpack of a Solder build, from its manifest. The manifest's Minecraft version wins,
/// and the pack's stands in when the manifest names none.
fn solder_modpack(
    name: String,
    fallback_minecraft: Option<String>,
    version: &ContentVersion,
    manifest: SolderManifest,
) -> Result<Modpack, PlatformError> {
    let minecraft = manifest.minecraft.or(fallback_minecraft).unwrap_or_default();
    let Some(forge) = manifest.forge.as_deref().filter(|forge| !forge.trim().is_empty()) else {
        return Err(PlatformError::UnsupportedPlatform {
            platform: Technic::NAME,
            action: "installing a pack whose build names no Forge version",
        });
    };
    let loader_version = combined_forge(&minecraft, forge);
    let files = manifest
        .mods
        .into_iter()
        .map(|mod_| ModpackFile {
            path: format!("mods/{}", file_name(&mod_.url)),
            url: mod_.url,
            sha1: None,
            size: mod_.filesize,
            project_id: None,
            file_id: None,
        })
        .collect();
    Ok(Modpack {
        name,
        version: Some(version.version_number.clone()),
        loader: Some("forge"),
        loader_version: Some(loader_version),
        minecraft,
        files,
        overrides: None,
        overrides_skip: Vec::new(),
        archives: Vec::new(),
    })
}

#[cfg(test)]
#[path = "../tests/technic.rs"]
mod tests;
