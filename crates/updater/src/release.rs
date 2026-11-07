//! The release that the update check reads from GitHub, the version comparison that decides whether it is newer, and the
//! file each system takes from a release.

use std::time::Duration;

use reqwest::StatusCode;
use reqwest::header::{ETAG, IF_NONE_MATCH};
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

/// The file a system takes its update from. Each name is the one `release.yml` gives the file, with the release's tag.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Payload {
    /// The Windows installer, `Lodestone-<tag>-setup.exe`.
    WindowsInstaller,
    /// The Linux build, `Lodestone-<tag>-linux-x86_64.tar.gz`, a gzipped tar of the executable.
    LinuxArchive,
    /// The macOS app, `Lodestone-<tag>-macos-universal.zip`, a zip of `Lodestone.app`.
    MacosArchive,
}

impl Payload {
    /// The file's name in a release with `tag`.
    pub fn file_name(self, tag: &str) -> String {
        match self {
            Self::WindowsInstaller => format!("Lodestone-{tag}-setup.exe"),
            Self::LinuxArchive => format!("Lodestone-{tag}-linux-x86_64.tar.gz"),
            Self::MacosArchive => format!("Lodestone-{tag}-macos-universal.zip"),
        }
    }
}

/// The answer to the release check.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Answer {
    /// A release newer than the running launcher.
    Newer {
        release: Release,
        /// The tag GitHub gave this answer, to send with the next check.
        etag: Option<String>,
    },
    /// The running launcher is the newest release on the channel.
    Latest { etag: Option<String> },
    /// GitHub's answer has not changed since the tag that was sent, so the last answer still holds.
    Unchanged,
}

/// Asks GitHub for the newest release on the channel: stable releases only, or prereleases as well. `etag` is the tag of
/// the last answer. When nothing has changed since then, GitHub sends no body and the answer is `Unchanged`.
pub async fn check(client: &reqwest::Client, current: &str, prereleases: bool, etag: Option<&str>) -> Result<Answer, Error> {
    let current = Version::parse(current).map_err(|_| Error::Version(current.to_string()))?;
    let url = if prereleases {
        format!("https://api.github.com/repos/{REPOSITORY}/releases?per_page=20")
    } else {
        format!("https://api.github.com/repos/{REPOSITORY}/releases/latest")
    };
    let mut request = client
        .get(url)
        .header(reqwest::header::ACCEPT, "application/vnd.github+json")
        .timeout(Duration::from_secs(20));
    if let Some(etag) = etag {
        request = request.header(IF_NONE_MATCH, etag);
    }
    let response = request.send().await?;
    if response.status() == StatusCode::NOT_MODIFIED {
        return Ok(Answer::Unchanged);
    }
    let etag = response.headers().get(ETAG).and_then(|value| value.to_str().ok()).map(str::to_owned);
    let response = response.error_for_status()?;
    let found = if prereleases {
        newest(&current, response.json::<Vec<Listed>>().await?)
    } else {
        newer(&current, response.json::<Release>().await?)?
    };
    Ok(match found {
        Some(release) => Answer::Newer { release, etag },
        None => Answer::Latest { etag },
    })
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

#[cfg(test)]
#[path = "../tests/release.rs"]
mod tests;
