//! Fabric loader — `meta.fabricmc.net` v2.

use std::path::{Path, PathBuf};

use serde::Deserialize;
use simple_download_utility::DownloadProgress;
use tokio::sync::mpsc::Sender;

use crate::LoaderKind;
use crate::error::LoaderError;
use crate::http::{download_files, get_json, maven_base, maven_jar_path};
use crate::install::{InstalledVersion, write_version};
use crate::meta::{LoaderVersion, ModInfo};
use crate::mod_info;
use crate::sealed;

/// Marker for the Fabric loader (https://fabricmc.net).
pub struct Fabric;

const META_BASE: &str = "https://meta.fabricmc.net/v2/versions/loader";
const DEFAULT_MAVEN: &str = "https://maven.fabricmc.net/";

/// One entry of the fabric-shaped loader meta APIs. Quilt's v3 shares this schema,
/// so its kind reuses these types and helpers with different base urls.
#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub(crate) struct Entry {
    /// The loader build itself.
    pub(crate) loader: Ref,
    /// Everything needed to place the loader's libraries on disk. Required: a
    /// meta entry without it is a malformed response and surfaces as a `Json`
    /// error.
    pub(crate) launcher_meta: LauncherMeta,
}

#[derive(Debug, Deserialize)]
pub(crate) struct Ref {
    pub(crate) version: String,
    /// Fabric v2 carries this field; quilt v3 does not (absent → stable).
    #[serde(default = "default_stable")]
    pub(crate) stable: bool,
    /// The loader jar's maven coordinate — the loader jar itself is NOT part of
    /// `launcherMeta.libraries`; it has to be fetched separately.
    #[serde(default)]
    pub(crate) maven: Option<String>,
}

fn default_stable() -> bool {
    true
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub(crate) struct LauncherMeta {
    pub(crate) libraries: Libraries,
}

#[derive(Debug, Deserialize)]
pub(crate) struct Libraries {
    #[serde(default)]
    pub(crate) client: Vec<Artifact>,
    #[serde(default)]
    pub(crate) common: Vec<Artifact>,
}

#[derive(Debug, Deserialize)]
pub(crate) struct Artifact {
    pub(crate) name: String,
    #[serde(default)]
    pub(crate) url: Option<String>,
}

/// The profile the meta API serves for one loader build: a version JSON that inherits
/// from the Minecraft version and lists the libraries the loader needs.
#[derive(Debug, Deserialize)]
pub(crate) struct Profile {
    /// The version id, such as `fabric-loader-0.16.10-1.21.4`.
    pub(crate) id: String,
    /// Every library the profile needs, the loader jar among them.
    #[serde(default)]
    pub(crate) libraries: Vec<Artifact>,
}

pub(crate) async fn list(http: &reqwest::Client, url: &str) -> Result<Vec<Entry>, LoaderError> {
    let json = get_json(http, url).await?;
    serde_json::from_value(json).map_err(|source| LoaderError::Json {
        url: url.to_string(),
        source,
    })
}

pub(crate) fn entry_version(entry: &Entry, minecraft: Option<&str>) -> LoaderVersion {
    LoaderVersion::new(entry.loader.version.clone(), minecraft.map(str::to_string), entry.loader.stable)
}

/// Finds one entry by loader version and downloads its client + common
/// libraries into `into`, returning the written paths.
///
/// Deliberately closure-free: closures capturing the surrounding method's
/// reference arguments trip rustc's RPITIT `+ Send` check ("implementation of
/// FnOnce is not general enough").
pub(crate) async fn download_libraries(
    entry: &Entry,
    default_maven: &str,
    into: &Path,
    progress: Option<&Sender<DownloadProgress>>,
) -> Result<Vec<PathBuf>, LoaderError> {
    let libraries = &entry.launcher_meta.libraries;
    let mut files = Vec::with_capacity(libraries.client.len() + libraries.common.len() + 1);
    // The loader jar comes first: it is referenced by `loader.maven`, not by the
    // launcherMeta library lists, and lives on the kind's default maven.
    if let Some(maven) = &entry.loader.maven
        && let Some(relative) = maven_jar_path(maven)
    {
        files.push((format!("{}{relative}", maven_base(default_maven)), into.join(native(relative))));
    }
    files.extend(artifact_files(libraries.client.iter().chain(&libraries.common), default_maven, into));
    download_files(files, progress).await
}

/// The `(url, destination)` pairs for `artifacts`, each downloaded from its own base url
/// and falling back to `default_maven`. Per-artifact base urls are authoritative: quilt
/// entries share libraries that live on maven.fabricmc.net. Names that are not maven
/// coordinates are skipped.
fn artifact_files<'a>(artifacts: impl IntoIterator<Item = &'a Artifact>, default_maven: &str, into: &Path) -> Vec<(String, PathBuf)> {
    artifacts
        .into_iter()
        .filter_map(|artifact| {
            let relative = maven_jar_path(&artifact.name)?;
            let base = maven_base(artifact.url.as_deref().unwrap_or(default_maven));
            Some((format!("{base}{relative}"), into.join(native(relative))))
        })
        .collect()
}

/// Installs a fabric-shaped loader from its profile. Fetches `<meta_base>/<mc>/<loader>/profile/json`,
/// downloads the profile's libraries into `game_dir/libraries`, and then writes the version JSON
/// under `game_dir/versions`.
pub(crate) async fn install_profile(
    http: &reqwest::Client,
    meta_base: &str,
    default_maven: &str,
    version: &LoaderVersion,
    game_dir: &Path,
    progress: Option<&Sender<DownloadProgress>>,
) -> Result<InstalledVersion, LoaderError> {
    let Some(minecraft) = version.minecraft.as_deref() else {
        return Err(LoaderError::MinecraftVersionUnknown {
            version: version.version.clone(),
        });
    };
    let url = format!("{meta_base}/{minecraft}/{}/profile/json", version.version);
    let json = get_json(http, &url).await?;
    let profile: Profile = serde_json::from_value(json.clone()).map_err(|source| LoaderError::Json { url: url.clone(), source })?;
    // The libraries land before the version JSON, so a JSON on disk means the install finished:
    // an interrupted install leaves no JSON for a launch to trust.
    let files = artifact_files(&profile.libraries, default_maven, &game_dir.join("libraries"));
    download_files(files, progress).await?;
    write_version(game_dir, &profile.id, &json)
}

/// Maven repository paths are slash-separated; destinations should use the
/// platform's separator so returned paths display cleanly.
fn native(relative: String) -> String {
    if std::path::MAIN_SEPARATOR == '/' {
        relative
    } else {
        relative.replace('/', std::path::MAIN_SEPARATOR_STR)
    }
}

impl sealed::Sealed for Fabric {}
impl LoaderKind for Fabric {
    const NAME: &'static str = "fabric";
    const SUPPORTS_MOD_INFO: bool = true;

    async fn list_versions(http: &reqwest::Client) -> Result<Vec<LoaderVersion>, LoaderError> {
        let entries = list(http, META_BASE).await?;
        Ok(entries.iter().map(|entry| entry_version(entry, None)).collect())
    }

    async fn versions_for_minecraft(http: &reqwest::Client, minecraft: &str) -> Result<Vec<LoaderVersion>, LoaderError> {
        let url = format!("{META_BASE}/{minecraft}");
        let entries = list(http, &url).await?;
        Ok(entries.iter().map(|entry| entry_version(entry, Some(minecraft))).collect())
    }

    async fn download_client(
        http: &reqwest::Client,
        version: &LoaderVersion,
        into: &Path,
        progress: Option<&Sender<DownloadProgress>>,
    ) -> Result<Vec<PathBuf>, LoaderError> {
        let Some(minecraft) = version.minecraft.as_deref() else {
            return Err(LoaderError::MinecraftVersionUnknown {
                version: version.version.clone(),
            });
        };
        let url = format!("{META_BASE}/{minecraft}");
        let entries = list(http, &url).await?;
        // A loop, not `iter().find(..)`: closures capturing the method's
        // reference arguments trip rustc's RPITIT `+ Send` check.
        let mut found = None;
        for entry in &entries {
            if entry.loader.version == version.version {
                found = Some(entry);
                break;
            }
        }
        let entry = match found {
            Some(entry) => entry,
            None => {
                return Err(LoaderError::VersionNotFound {
                    version: version.version.clone(),
                    minecraft: minecraft.to_string(),
                });
            }
        };
        download_libraries(entry, DEFAULT_MAVEN, into, progress).await
    }

    async fn install(
        http: &reqwest::Client,
        version: &LoaderVersion,
        game_dir: &Path,
        _java: &Path,
        _lines: Option<&Sender<String>>,
        progress: Option<&Sender<DownloadProgress>>,
    ) -> Result<InstalledVersion, LoaderError> {
        install_profile(http, META_BASE, DEFAULT_MAVEN, version, game_dir, progress).await
    }

    fn extract_mod_info(jar: &Path) -> Result<ModInfo, LoaderError> {
        mod_info::extract(jar)
    }
}

#[cfg(test)]
#[path = "../tests/fabric.rs"]
mod tests;
