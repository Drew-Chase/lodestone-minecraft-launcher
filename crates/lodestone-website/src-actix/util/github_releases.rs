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
    download_count: u64,
}

#[derive(Debug, Clone, Deserialize)]
struct GhRepo {
    stargazers_count: u64,
    forks_count: u64,
    open_issues_count: u64,
    license: Option<GhLicense>,
}

#[derive(Debug, Clone, Deserialize)]
struct GhLicense {
    spdx_id: Option<String>,
    name: Option<String>,
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
    pub download_count: u64,
}

#[derive(Debug, Clone, Serialize)]
pub struct RepoStats {
    pub stars: u64,
    pub total_downloads: u64,
    pub forks: u64,
    pub open_issues: u64,
    pub license: Option<String>,
    pub license_name: Option<String>,
    pub contributors: u64,
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
    repo: GhRepo,
    contributors: u64,
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

    async fn ensure_cache(&self) -> anyhow::Result<()> {
        // Check if cache is still valid
        {
            let guard = self.inner.read().await;
            if let Some(entry) = guard.as_ref()
                && entry.fetched_at.elapsed() < CACHE_TTL
            {
                return Ok(());
            }
        }

        // Cache miss or expired — fetch from GitHub
        let (releases, repo, contributors) = tokio::try_join!(
            self.fetch_releases(),
            self.fetch_repo(),
            self.fetch_contributor_count(),
        )?;

        // Update cache
        {
            let mut guard = self.inner.write().await;
            *guard = Some(CacheEntry {
                releases,
                repo,
                contributors,
                fetched_at: Instant::now(),
            });
        }

        Ok(())
    }

    pub async fn get_releases(&self) -> anyhow::Result<Vec<Release>> {
        self.ensure_cache().await?;
        let guard = self.inner.read().await;
        Ok(guard.as_ref().unwrap().releases.clone())
    }

    pub async fn get_stats(&self) -> anyhow::Result<RepoStats> {
        self.ensure_cache().await?;
        let guard = self.inner.read().await;
        let entry = guard.as_ref().unwrap();
        let total_downloads: u64 = entry.releases.iter()
            .flat_map(|r| &r.assets)
            .map(|a| a.download_count)
            .sum();
        Ok(RepoStats {
            stars: entry.repo.stargazers_count,
            total_downloads,
            forks: entry.repo.forks_count,
            open_issues: entry.repo.open_issues_count,
            license: entry.repo.license.as_ref().and_then(|l| l.spdx_id.clone()),
            license_name: entry.repo.license.as_ref().and_then(|l| l.name.clone()),
            contributors: entry.contributors,
        })
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
                            download_count: a.download_count,
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

    async fn fetch_repo(&self) -> anyhow::Result<GhRepo> {
        let url = format!(
            "https://api.github.com/repos/{}/{}",
            GITHUB_OWNER, GITHUB_REPO
        );

        log::info!("Fetching GitHub repo info from {}", url);

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

        let repo: GhRepo = resp.json().await?;
        log::info!("GitHub stars: {}, forks: {}", repo.stargazers_count, repo.forks_count);
        Ok(repo)
    }

    /// Fetch contributor count using the per_page=1 trick — the last page
    /// number from the `Link` header gives the total count without
    /// downloading every contributor object.
    async fn fetch_contributor_count(&self) -> anyhow::Result<u64> {
        let url = format!(
            "https://api.github.com/repos/{}/{}/contributors?per_page=1&anon=true",
            GITHUB_OWNER, GITHUB_REPO
        );

        log::info!("Fetching GitHub contributor count");

        let resp = self.client.get(&url).send().await?;

        if !resp.status().is_success() {
            // Non-critical — return 0 rather than failing the whole cache refresh
            log::warn!("Failed to fetch contributor count: {}", resp.status());
            return Ok(0);
        }

        // Parse the Link header: <...?page=N>; rel="last"
        if let Some(link) = resp.headers().get("link").and_then(|v| v.to_str().ok()) {
            for part in link.split(',') {
                if part.contains("rel=\"last\"")
                    && let Some(page) = part
                        .split("page=")
                        .last()
                        .and_then(|s| s.split('>').next())
                        .and_then(|s| s.parse::<u64>().ok())
                {
                    log::info!("GitHub contributors: {}", page);
                    return Ok(page);
                }
            }
        }

        // If there's no Link header, the total fits in one page
        // Count the items in the response body
        let items: Vec<serde_json::Value> = resp.json().await.unwrap_or_default();
        Ok(items.len() as u64)
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
