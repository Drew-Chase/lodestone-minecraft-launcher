//! What the New Instance form lists: Mojang's release versions, the Java runtime a version
//! asks for, the bundled runtimes for this platform, and a loader's builds for one
//! Minecraft version.

use std::cmp::Ordering;

use lodestone_loaders::{self as loaders, LoaderKind, LoaderVersion};
use piston_mc::manifest_v2::ManifestV2;
use piston_mc::version_manifest::VersionManifest;

use crate::app::lang;
use crate::app::pages::library::instance::Loader;

/// Mojang's release versions, newest first, with the one it marks as the latest. The
/// manifest is kept so a version's own JSON can be fetched later.
#[derive(Debug, Clone)]
pub struct Releases {
    pub ids: Vec<String>,
    pub latest: Option<String>,
    pub manifest: ManifestV2,
}

/// The Java runtime a version asks for. `major` is missing on the oldest versions.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct VersionInfo {
    pub component: String,
    pub major: Option<u8>,
}

/// A bundled Java runtime the platform offers: its component and a label such as
/// "Java 21 · delta".
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct BundledJava {
    pub component: String,
    pub label: String,
}

/// Fetches Mojang's version list.
pub async fn releases() -> Result<Releases, String> {
    let manifest = ManifestV2::fetch()
        .await
        .map_err(|error| lang::t_with("install.version_list_failed", &[("error", error.to_string().as_str())]))?;
    let ids = manifest.releases().into_iter().map(|version| version.id).collect();
    let latest = Some(manifest.latest.release.clone());
    Ok(Releases { ids, latest, manifest })
}

/// Fetches the version's JSON and reads its Java requirement.
pub async fn version_info(releases: &Releases, mc: &str) -> Result<VersionInfo, String> {
    let manifest: Option<VersionManifest> = releases
        .manifest
        .version(mc)
        .await
        .map_err(|error| lang::t_with("install.version_fetch_failed", &[("mc", mc), ("error", error.to_string().as_str())]))?;
    let version = manifest.ok_or_else(|| lang::t_with("install.version_missing", &[("mc", mc)]))?;
    Ok(VersionInfo {
        component: version
            .java_version
            .as_ref()
            .map_or_else(|| "jre-legacy".to_string(), |java| java.component.clone()),
        major: version.java_version.as_ref().map(|java| java.major_version),
    })
}

/// The runtimes this platform's manifest lists, newest Java first (the order
/// `platform_components` gives).
pub async fn bundled_java() -> Result<Vec<BundledJava>, String> {
    let runtimes = super::java::platform_components().await?;
    let bundled: Vec<BundledJava> = runtimes
        .into_iter()
        .map(|(component, major)| BundledJava {
            label: match major {
                Some(major) => lang::t_with(
                    "install.java_option",
                    &[("major", major.to_string().as_str()), ("runtime", super::java::short_name(&component))],
                ),
                None => super::java::short_name(&component).to_string(),
            },
            component,
        })
        .collect();
    Ok(bundled)
}

/// The builds a loader offers for one Minecraft version, the newest first. Stable builds
/// are not moved ahead of prerelease ones here: callers pick the first stable entry.
pub async fn loader_builds(loader: Loader, mc: &str) -> Result<Vec<LoaderVersion>, String> {
    let mut builds = match loader {
        Loader::Vanilla => Vec::new(),
        Loader::Fabric => builds_for::<loaders::Fabric>(mc).await?,
        Loader::Forge => builds_for::<loaders::Forge>(mc).await?,
        Loader::Quilt => builds_for::<loaders::Quilt>(mc).await?,
        Loader::NeoForge => builds_for::<loaders::NeoForge>(mc).await?,
    };
    builds.sort_by(|a, b| natural_cmp(&b.version, &a.version));
    Ok(builds)
}

async fn builds_for<K: LoaderKind>(mc: &str) -> Result<Vec<LoaderVersion>, String> {
    loaders::Loader::<K>::new()
        .versions_for_minecraft(mc)
        .await
        .map_err(|error| error.to_string())
}

/// Orders version strings by their numeric parts, so `47.10.0` comes after `47.9.0`.
pub fn natural_cmp(a: &str, b: &str) -> Ordering {
    let mut left = chunks(a);
    let mut right = chunks(b);
    loop {
        match (left.next(), right.next()) {
            (None, None) => return Ordering::Equal,
            (None, Some(_)) => return Ordering::Less,
            (Some(_), None) => return Ordering::Greater,
            (Some(x), Some(y)) => {
                let ordering = match (x.parse::<u64>(), y.parse::<u64>()) {
                    (Ok(x), Ok(y)) => x.cmp(&y),
                    _ => x.cmp(y),
                };
                if ordering != Ordering::Equal {
                    return ordering;
                }
            }
        }
    }
}

/// Splits a version into runs of digits and runs of everything else.
fn chunks(version: &str) -> impl Iterator<Item = &str> {
    let mut rest = version;
    std::iter::from_fn(move || {
        let first = rest.chars().next()?;
        let digit = first.is_ascii_digit();
        let end = rest.find(|c: char| c.is_ascii_digit() != digit).unwrap_or(rest.len());
        let (chunk, tail) = rest.split_at(end);
        rest = tail;
        Some(chunk)
    })
    .filter(|chunk| !chunk.is_empty())
}

#[cfg(test)]
#[path = "../../../tests/app/install/catalog.rs"]
mod tests;
