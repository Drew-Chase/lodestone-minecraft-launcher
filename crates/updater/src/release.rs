//! The release that the update check reads from GitHub, and the version comparison that decides whether it is newer.

use std::time::Duration;

use semver::Version;
use serde::{Deserialize, Deserializer};

use crate::REPOSITORY;
use crate::error::Error;

/// A GitHub release, as far as the update check reads it.
#[derive(Debug, Clone, PartialEq, Eq, Deserialize)]
pub struct Release {
    /// The release's tag, such as `v0.2.0`.
    #[serde(rename = "tag_name")]
    pub tag: String,
    /// The release's page on GitHub.
    pub html_url: String,
    /// The release's notes, as GitHub's markdown. Empty when the release has none, which GitHub sends as `null`.
    #[serde(default, deserialize_with = "notes_or_empty")]
    pub body: String,
    /// The files attached to the release.
    #[serde(default)]
    pub assets: Vec<Asset>,
}

/// A file attached to a release.
#[derive(Debug, Clone, PartialEq, Eq, Deserialize)]
pub struct Asset {
    /// The file's name, such as `Lodestone-v0.2.0-setup.exe`.
    pub name: String,
    /// Where the file is downloaded from.
    pub browser_download_url: String,
}

/// Reads a release's notes, where a release with none sends `null`.
fn notes_or_empty<'de, D: Deserializer<'de>>(deserializer: D) -> Result<String, D::Error> {
    Option::<String>::deserialize(deserializer).map(Option::unwrap_or_default)
}

impl Release {
    /// The version that the tag names, without its leading `v`.
    pub fn version(&self) -> Result<Version, Error> {
        let text = self.tag.strip_prefix('v').unwrap_or(&self.tag);
        Version::parse(text).map_err(|_| Error::Version(self.tag.clone()))
    }
}

/// The newest release, when it is newer than `current`, a version such as `0.1.0`. GitHub's `releases/latest` skips
/// drafts and prereleases, so only published releases reach this check.
pub async fn latest(client: &reqwest::Client, current: &str) -> Result<Option<Release>, Error> {
    let current = Version::parse(current).map_err(|_| Error::Version(current.to_string()))?;
    let url = format!("https://api.github.com/repos/{REPOSITORY}/releases/latest");
    let release = client
        .get(url)
        .header(reqwest::header::ACCEPT, "application/vnd.github+json")
        .timeout(Duration::from_secs(20))
        .send()
        .await?
        .error_for_status()?
        .json::<Release>()
        .await?;
    newer(&current, release)
}

/// The newest release, prereleases included, when it is newer than `current`. Drafts are never offered.
pub async fn latest_with_prereleases(client: &reqwest::Client, current: &str) -> Result<Option<Release>, Error> {
    let current = Version::parse(current).map_err(|_| Error::Version(current.to_string()))?;
    let url = format!("https://api.github.com/repos/{REPOSITORY}/releases?per_page=20");
    let listed = client
        .get(url)
        .header(reqwest::header::ACCEPT, "application/vnd.github+json")
        .timeout(Duration::from_secs(20))
        .send()
        .await?
        .error_for_status()?
        .json::<Vec<Listed>>()
        .await?;
    Ok(newest(&current, listed))
}

/// The newest release on the channel the caller chose: stable releases only, or prereleases as well.
pub async fn latest_for(client: &reqwest::Client, current: &str, prereleases: bool) -> Result<Option<Release>, Error> {
    if prereleases {
        latest_with_prereleases(client, current).await
    } else {
        latest(client, current).await
    }
}

/// The release in `listed` with the highest version, when that version is newer than `current`. Drafts are skipped.
fn newest(current: &Version, listed: Vec<Listed>) -> Option<Release> {
    let mut best: Option<(Version, Release)> = None;
    for entry in listed.into_iter().filter(|entry| !entry.draft) {
        let Ok(version) = entry.release.version() else {
            continue;
        };
        if best.as_ref().is_none_or(|(highest, _)| version > *highest) {
            best = Some((version, entry.release));
        }
    }
    best.and_then(|(version, release)| (version > *current).then_some(release))
}

/// A release as the release list returns it, with the flag the list adds.
#[derive(Debug, Deserialize)]
struct Listed {
    #[serde(flatten)]
    release: Release,
    #[serde(default)]
    draft: bool,
}

/// The release, when its version is newer than `current`.
pub fn newer(current: &Version, release: Release) -> Result<Option<Release>, Error> {
    let version = release.version()?;
    Ok((version > *current).then_some(release))
}

/// The release's installer for this system: `installer.sh` on Linux, the `-setup.exe` on Windows.
pub fn installer_asset(release: &Release) -> Option<&Asset> {
    release.assets.iter().find(|asset| is_installer(&asset.name))
}

#[cfg(target_os = "linux")]
fn is_installer(name: &str) -> bool {
    name == "installer.sh"
}

#[cfg(not(target_os = "linux"))]
fn is_installer(name: &str) -> bool {
    name.ends_with("-setup.exe")
}

#[cfg(test)]
#[path = "../tests/release.rs"]
mod tests;
