//! ATLauncher — hand-rolled binding for `api.atlauncher.com/v1`.
//!
//! Pack codes are the platform's ids (e.g. `SkyFactory`). The API has no
//! search or pack-list endpoint, so `search` reports
//! [`PlatformError::UnsupportedPlatform`], and pack downloads assemble from
//! version data at install time, so `download` reports it too.

use std::path::Path;

use serde::Deserialize;
use simple_download_utility::DownloadProgress;
use tokio::sync::mpsc::Sender;

use crate::PlatformKind;
use crate::error::PlatformError;
use crate::http::get_json;
use crate::meta::{ContentDetails, ContentType, ContentVersion, SearchResult};
use crate::sealed;

/// Marker for ATLauncher (https://atlauncher.com).
pub struct AtLauncher;

const BASE: &str = "https://api.atlauncher.com/v1";

/// The ATLauncher API envelope: `{ error, code, message, data }`.
#[derive(Debug, Deserialize)]
struct Envelope<T> {
    data: Option<T>,
}

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

impl Pack {
    fn into_details(self, id: &str) -> ContentDetails {
        ContentDetails {
            id: id.to_string(),
            slug: self.safe_name,
            name: self.name,
            author: Some("ATLauncher".to_string()),
            summary: self.description.clone().unwrap_or_default(),
            description: self.description,
            downloads: 0,
            icon_url: None,
            web_url: None,
            content_type: ContentType::Modpack,
        }
    }
}

impl sealed::Sealed for AtLauncher {}
impl PlatformKind for AtLauncher {
    const NAME: &'static str = "atlauncher";
    const CONTENT_TYPES: &'static [ContentType] = &[ContentType::Modpack];
    const SUPPORTS_MODPACK_ARCHIVE: bool = false;

    async fn search(
        _http: &reqwest::Client,
        _credential: Option<&str>,
        _query: &str,
        _content: ContentType,
    ) -> Result<Vec<SearchResult>, PlatformError> {
        Err(PlatformError::UnsupportedPlatform {
            platform: Self::NAME,
            action: "content search",
        })
    }

    async fn details(http: &reqwest::Client, _credential: Option<&str>, id: &str, _content: ContentType) -> Result<ContentDetails, PlatformError> {
        let url = format!("{BASE}/pack/{id}");
        let response = get_json(http, &url).await?;
        let envelope: Envelope<Pack> = serde_json::from_value(response).map_err(|source| PlatformError::Json { url: url.clone(), source })?;
        let pack = envelope.data.ok_or_else(|| PlatformError::UnexpectedResponse {
            url: url.clone(),
            status: 200,
        })?;
        Ok(pack.into_details(id))
    }

    async fn versions(
        http: &reqwest::Client,
        _credential: Option<&str>,
        id: &str,
        _content: ContentType,
    ) -> Result<Vec<ContentVersion>, PlatformError> {
        let url = format!("{BASE}/pack/{id}");
        let response = get_json(http, &url).await?;
        let envelope: Envelope<Pack> = serde_json::from_value(response).map_err(|source| PlatformError::Json { url: url.clone(), source })?;
        let pack = envelope.data.ok_or_else(|| PlatformError::UnexpectedResponse {
            url: url.clone(),
            status: 200,
        })?;
        Ok(pack
            .versions
            .into_iter()
            .map(|version| ContentVersion {
                id: version.version.clone(),
                project_id: None,
                name: Some(version.version.clone()),
                version_number: version.version,
                content_type: ContentType::Modpack,
                minecraft_versions: vec![version.minecraft_version],
                loaders: Vec::new(),
                downloads: 0,
                date: version.published.map(|published| published.to_string()),
                changelog_url: None,
                // ATLauncher packs assemble from version data at install time;
                // there is no single-file download.
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
    ) -> Result<Vec<std::path::PathBuf>, PlatformError> {
        Err(PlatformError::UnsupportedPlatform {
            platform: Self::NAME,
            action: "pack download (packs assemble from version data at install time)",
        })
    }

    fn parse_modpack(_archive: &Path) -> Result<crate::meta::Modpack, PlatformError> {
        Err(PlatformError::UnsupportedPlatform {
            platform: Self::NAME,
            action: "modpack archive parsing",
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Recorded slice of the real `/v1/pack/SkyFactory` response.
    const PACK: &str = r#"{
        "error": false,
        "code": 200,
        "message": null,
        "data": {
            "id": 190,
            "name": "Sky Factory",
            "safeName": "SkyFactory",
            "type": "public",
            "description": "A skyblock pack",
            "versions": [
                { "version": "2.4.1", "minecraft": "1.7.10", "published": 1639190283 },
                { "version": "2.4", "minecraft": "1.7.10", "published": 1639190283 }
            ]
        }
    }"#;

    #[test]
    fn the_pack_envelope_parses() {
        let envelope: Envelope<Pack> = serde_json::from_str(PACK).expect("recorded payload parses");
        let pack = envelope.data.expect("data present");
        assert_eq!(pack.name, "Sky Factory");
        assert_eq!(pack.safe_name.as_deref(), Some("SkyFactory"));
        assert_eq!(pack.versions.len(), 2);
        assert_eq!(pack.versions[0].version, "2.4.1");
        assert_eq!(pack.versions[0].minecraft_version, "1.7.10");
    }

    #[test]
    fn versions_map_with_their_minecraft_version() {
        let envelope: Envelope<Pack> = serde_json::from_str(PACK).expect("recorded payload parses");
        let pack = envelope.data.expect("data present");
        let versions: Vec<ContentVersion> = pack
            .versions
            .into_iter()
            .map(|version| ContentVersion {
                id: version.version.clone(),
                project_id: None,
                name: Some(version.version.clone()),
                version_number: version.version,
                content_type: ContentType::Modpack,
                minecraft_versions: vec![version.minecraft_version],
                loaders: Vec::new(),
                downloads: 0,
                date: version.published.map(|published| published.to_string()),
                changelog_url: None,
                files: Vec::new(),
            })
            .collect();
        assert_eq!(versions.len(), 2);
        assert_eq!(versions[0].minecraft_versions, ["1.7.10"]);
    }
}
