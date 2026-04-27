use serde::{Deserialize, Serialize};
use std::sync::Arc;
use std::time::Duration;
use tokio::sync::RwLock;
use tokio::time::Instant;

const GITHUB_OWNER: &str = "drew-chase";
const GITHUB_REPO: &str = "lodestone-minecraft-launcher";
const CACHE_TTL: Duration = Duration::from_secs(30 * 60); // 30 minutes

// ─── GitHub API response types ───────────────────────────────────────────────

#[derive(Debug, Clone, Deserialize)]
struct GhAsset {
    name: String,
    browser_download_url: String,
    size: u64,
    content_type: String,
}

#[derive(Debug, Clone, Deserialize)]
struct GhRelease {
    tag_name: String,
    name: Option<String>,
    body: Option<String>,
    draft: bool,
    prerelease: bool,
    published_at: Option<String>,
    html_url: String,
    assets: Vec<GhAsset>,
}

// ─── Public API types ────────────────────────────────────────────────────────

#[derive(Debug, Clone, Serialize)]
pub struct ReleaseAsset {
    pub name: String,
    pub download_url: String,
    pub size: u64,
    pub content_type: String,
    pub platform: Option<String>,
}

#[derive(Debug, Clone, Serialize)]
pub struct Release {
    pub version: String,
    pub tag: String,
    pub name: String,
    pub changelog: String,
    pub prerelease: bool,
    pub published_at: Option<String>,
    pub html_url: String,
    pub assets: Vec<ReleaseAsset>,
}

#[derive(Debug, Clone, Serialize)]
pub struct PlatformDownload {
    pub platform: String,
    pub label: String,
    pub assets: Vec<ReleaseAsset>,
}

#[derive(Debug, Clone, Serialize)]
pub struct LatestRelease {
    pub release: Release,
    pub downloads: Vec<PlatformDownload>,
}

// ─── Cache ───────────────────────────────────────────────────────────────────

struct CacheEntry {
    releases: Vec<Release>,
    fetched_at: Instant,
}

pub struct ReleasesCache {
    inner: RwLock<Option<CacheEntry>>,
    client: reqwest::Client,
}

impl ReleasesCache {
    pub fn new() -> Arc<Self> {
        let client = reqwest::Client::builder()
            .user_agent("lodestone-website/1.0")
            .timeout(Duration::from_secs(15))
            .build()
            .expect("Failed to build HTTP client");

        Arc::new(Self {
            inner: RwLock::new(None),
            client,
        })
    }

    pub async fn get_releases(&self) -> anyhow::Result<Vec<Release>> {
        // Check if cache is still valid
        {
            let guard = self.inner.read().await;
            if let Some(entry) = guard.as_ref()
                && entry.fetched_at.elapsed() < CACHE_TTL
            {
                return Ok(entry.releases.clone());
            }
        }

        // Cache miss or expired — fetch from GitHub
        let releases = self.fetch_releases().await?;

        // Update cache
        {
            let mut guard = self.inner.write().await;
            *guard = Some(CacheEntry {
                releases: releases.clone(),
                fetched_at: Instant::now(),
            });
        }

        Ok(releases)
    }

    pub async fn get_latest(&self) -> anyhow::Result<Option<LatestRelease>> {
        let releases = self.get_releases().await?;
        let release = match releases.into_iter().find(|r| !r.prerelease) {
            Some(r) => r,
            None => return Ok(None),
        };

        let downloads = build_platform_downloads(&release.assets);

        Ok(Some(LatestRelease { release, downloads }))
    }

    async fn fetch_releases(&self) -> anyhow::Result<Vec<Release>> {
        let url = format!(
            "https://api.github.com/repos/{}/{}/releases?per_page=25",
            GITHUB_OWNER, GITHUB_REPO
        );

        log::info!("Fetching GitHub releases from {}", url);

        let resp = self.client.get(&url).send().await?;

        if !resp.status().is_success() {
            let status = resp.status();
            let body = resp.text().await.unwrap_or_default();
            anyhow::bail!(
                "GitHub API returned {}: {}",
                status,
                &body[..body.len().min(200)]
            );
        }

        let gh_releases: Vec<GhRelease> = resp.json().await?;

        let releases: Vec<Release> = gh_releases
            .into_iter()
            .filter(|r| !r.draft)
            .map(|r| {
                let version = r.tag_name.strip_prefix('v').unwrap_or(&r.tag_name).to_string();
                let assets: Vec<ReleaseAsset> = r
                    .assets
                    .into_iter()
                    .map(|a| {
                        let platform = detect_platform(&a.name);
                        ReleaseAsset {
                            name: a.name,
                            download_url: a.browser_download_url,
                            size: a.size,
                            content_type: a.content_type,
                            platform,
                        }
                    })
                    .collect();

                Release {
                    version,
                    tag: r.tag_name,
                    name: r.name.unwrap_or_default(),
                    changelog: r.body.unwrap_or_default(),
                    prerelease: r.prerelease,
                    published_at: r.published_at,
                    html_url: r.html_url,
                    assets,
                }
            })
            .collect();

        log::info!("Cached {} releases from GitHub", releases.len());
        Ok(releases)
    }
}

// ─── Platform detection ──────────────────────────────────────────────────────

fn detect_platform(filename: &str) -> Option<String> {
    let lower = filename.to_lowercase();

    if lower.ends_with(".msi") || lower.ends_with("-setup.exe") || lower.ends_with("setup.nsis.zip") {
        return Some("windows".into());
    }
    if lower == "lodestone-launcher.exe" {
        return Some("windows-portable".into());
    }

    if lower.ends_with(".dmg") {
        return Some("macos".into());
    }

    if lower.ends_with(".deb") {
        return Some("linux-deb".into());
    }
    if lower.ends_with(".rpm") {
        return Some("linux-rpm".into());
    }
    if lower.ends_with(".appimage") {
        return Some("linux-appimage".into());
    }
    if lower == "lodestone-launcher" {
        return Some("linux-portable".into());
    }

    // Tauri updater signature files
    if lower.ends_with(".sig") {
        return Some("signature".into());
    }

    None
}

fn build_platform_downloads(assets: &[ReleaseAsset]) -> Vec<PlatformDownload> {
    let platforms = [
        ("windows", "Windows", vec!["windows"]),
        ("macos", "macOS", vec!["macos"]),
        ("linux", "Linux", vec!["linux-deb", "linux-rpm", "linux-appimage", "linux-portable"]),
    ];

    platforms
        .iter()
        .filter_map(|(key, label, match_prefixes)| {
            let matched: Vec<ReleaseAsset> = assets
                .iter()
                .filter(|a| {
                    if let Some(p) = &a.platform {
                        match_prefixes.iter().any(|prefix| p.starts_with(prefix))
                    } else {
                        false
                    }
                })
                .cloned()
                .collect();

            if matched.is_empty() {
                None
            } else {
                Some(PlatformDownload {
                    platform: key.to_string(),
                    label: label.to_string(),
                    assets: matched,
                })
            }
        })
        .collect()
}
