//! The release that the update check reads from GitHub, and the version comparison that decides whether it is newer.

use std::time::Duration;

use semver::Version;
use serde::Deserialize;

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

/// The release, when its version is newer than `current`.
pub fn newer(current: &Version, release: Release) -> Result<Option<Release>, Error> {
    let version = release.version()?;
    Ok((version > *current).then_some(release))
}

/// The release's Windows installer: the asset whose name ends in `-setup.exe`.
pub fn installer_asset(release: &Release) -> Option<&Asset> {
    release.assets.iter().find(|asset| asset.name.ends_with("-setup.exe"))
}

#[cfg(test)]
#[path = "../tests/release.rs"]
mod tests;
