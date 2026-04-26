use anyhow::{anyhow, Result};
use serde::{Deserialize, Serialize};

const VERSIONS_URL: &str =
    "https://maven.neoforged.net/api/maven/versions/releases/net/neoforged/neoforge";

/// Contains all available NeoForge versions.
///
/// NeoForge version strings encode the Minecraft version they target.
/// For example, NeoForge `21.1.77` targets Minecraft `1.21.1`.
/// The mapping is: NeoForge `major.minor.patch` → MC `1.{major}.{minor}`,
/// where minor `0` maps to MC `1.{major}` (no trailing `.0`).
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct NeoForgeVersions {
    /// Raw version strings from the Maven API.
    pub versions: Vec<String>,
}

#[derive(Debug, Deserialize)]
struct MavenResponse {
    versions: Vec<String>,
}

impl NeoForgeVersions {
    /// Fetch all available NeoForge versions from the Maven API.
    pub async fn fetch() -> Result<Self> {
        let response = reqwest::get(VERSIONS_URL).await?;
        if !response.status().is_success() {
            return Err(anyhow!(
                "Failed to fetch NeoForge versions: HTTP {}",
                response.status()
            ));
        }
        let data: MavenResponse = response.json().await?;
        Ok(Self {
            versions: data.versions,
        })
    }

    /// Get all NeoForge versions for a specific Minecraft version.
    ///
    /// Returns versions in the order they appear (typically newest last).
    pub fn get_versions(&self, mc_version: &str) -> Vec<&str> {
        let prefix = mc_to_neoforge_prefix(mc_version);
        match prefix {
            Some(p) => self
                .versions
                .iter()
                .filter(|v| v.starts_with(&p))
                .map(|v| v.as_str())
                .collect(),
            None => Vec::new(),
        }
    }

    /// Get the latest (highest) NeoForge version for a Minecraft version.
    pub fn get_latest(&self, mc_version: &str) -> Option<&str> {
        self.get_versions(mc_version).into_iter().last()
    }

    /// Get all Minecraft versions that have NeoForge builds available.
    pub fn get_supported_minecraft_versions(&self) -> Vec<String> {
        let mut mc_versions: Vec<String> = self
            .versions
            .iter()
            .filter_map(|v| neoforge_to_mc(v))
            .collect::<std::collections::HashSet<_>>()
            .into_iter()
            .collect();
        mc_versions.sort_by(|a, b| compare_mc_versions(b, a));
        mc_versions
    }

    /// Build the installer JAR download URL for a given NeoForge version.
    pub fn installer_url(neoforge_version: &str) -> String {
        format!(
            "https://maven.neoforged.net/releases/net/neoforged/neoforge/{v}/neoforge-{v}-installer.jar",
            v = neoforge_version
        )
    }
}

/// Convert a Minecraft version like `1.21.1` to a NeoForge prefix like `21.1.`.
/// MC `1.21` (no patch) maps to prefix `21.0.`.
fn mc_to_neoforge_prefix(mc_version: &str) -> Option<String> {
    let parts: Vec<&str> = mc_version.split('.').collect();
    if parts.len() < 2 || parts[0] != "1" {
        return None;
    }
    let major = parts[1];
    let minor = if parts.len() >= 3 { parts[2] } else { "0" };
    Some(format!("{major}.{minor}."))
}

/// Convert a NeoForge version like `21.1.77` to its Minecraft version `1.21.1`.
fn neoforge_to_mc(nf_version: &str) -> Option<String> {
    // Strip beta/rc suffixes for parsing
    let clean = nf_version.split('-').next().unwrap_or(nf_version);
    let parts: Vec<&str> = clean.split('.').collect();
    if parts.len() < 2 {
        return None;
    }
    let major = parts[0];
    let minor = parts[1];
    if minor == "0" {
        Some(format!("1.{major}"))
    } else {
        Some(format!("1.{major}.{minor}"))
    }
}

/// Compare Minecraft version strings for sorting (e.g., "1.21.1" > "1.20.4").
fn compare_mc_versions(a: &str, b: &str) -> std::cmp::Ordering {
    let parse = |v: &str| -> Vec<u32> {
        v.split('.').filter_map(|p| p.parse().ok()).collect()
    };
    parse(a).cmp(&parse(b))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_mc_to_neoforge_prefix() {
        assert_eq!(mc_to_neoforge_prefix("1.21.1"), Some("21.1.".into()));
        assert_eq!(mc_to_neoforge_prefix("1.21"), Some("21.0.".into()));
        assert_eq!(mc_to_neoforge_prefix("1.20.4"), Some("20.4.".into()));
        assert_eq!(mc_to_neoforge_prefix("2.0"), None);
    }

    #[test]
    fn test_neoforge_to_mc() {
        assert_eq!(neoforge_to_mc("21.1.77"), Some("1.21.1".into()));
        assert_eq!(neoforge_to_mc("21.0.14"), Some("1.21".into()));
        assert_eq!(neoforge_to_mc("20.4.237"), Some("1.20.4".into()));
        assert_eq!(neoforge_to_mc("21.1.77-beta"), Some("1.21.1".into()));
    }

    #[tokio::test]
    async fn test_fetch_versions() {
        let versions = NeoForgeVersions::fetch().await;
        assert!(versions.is_ok());
        let versions = versions.unwrap();
        assert!(!versions.versions.is_empty());
    }

    #[tokio::test]
    async fn test_get_versions_for_mc() {
        let versions = NeoForgeVersions::fetch().await.unwrap();
        let nf_versions = versions.get_versions("1.21.1");
        assert!(!nf_versions.is_empty());
        for v in &nf_versions {
            assert!(v.starts_with("21.1."));
        }
    }

    #[tokio::test]
    async fn test_supported_mc_versions() {
        let versions = NeoForgeVersions::fetch().await.unwrap();
        let mc = versions.get_supported_minecraft_versions();
        assert!(!mc.is_empty());
        assert!(mc.iter().any(|v| v.starts_with("1.2")));
    }
}
