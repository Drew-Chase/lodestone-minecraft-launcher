//! CurseForge's fingerprint lookup: the file a jar's fingerprint names, when CurseForge has one.

use serde::Deserialize;

use super::{MINECRAFT_GAME_ID, request};
use crate::error::PlatformError;
use crate::http::post_json;

/// The CurseForge file a fingerprint names: the file's project, and the file's own id. Both are CurseForge's ids as text.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct FingerprintMatch {
    /// The project's id on CurseForge.
    pub project_id: String,
    /// The file's id on CurseForge.
    pub file_id: String,
}

/// The fingerprint endpoint's answer. Only the exact matches are read: each names a file by its own record.
#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
struct CfFingerprintEnvelope {
    data: CfFingerprintData,
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
struct CfFingerprintData {
    #[serde(default)]
    exact_matches: Vec<CfFingerprintMatch>,
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
struct CfFingerprintMatch {
    #[serde(default)]
    file: Option<CfFingerprintFile>,
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
struct CfFingerprintFile {
    id: u64,
    mod_id: u64,
}

/// The CurseForge file a fingerprint names, when CurseForge has one. It asks about one fingerprint at a time, so every exact
/// match in the answer is a match for it. Reached through `Platform::<CurseForge>::fingerprint_match`.
pub(crate) async fn fingerprint_match(credential: Option<&str>, fingerprint: u32) -> Result<Option<FingerprintMatch>, PlatformError> {
    let (url, client) = request(credential, &format!("/fingerprints/{MINECRAFT_GAME_ID}"))?;
    let body = serde_json::json!({ "fingerprints": [fingerprint] });
    let response = post_json(&client, &url, &body).await?;
    fingerprint_match_of(response, &url)
}

/// The first exact match of a fingerprint answer that names its file, or `None` when CurseForge names none.
pub(super) fn fingerprint_match_of(response: serde_json::Value, url: &str) -> Result<Option<FingerprintMatch>, PlatformError> {
    let envelope: CfFingerprintEnvelope = serde_json::from_value(response).map_err(|source| PlatformError::Json {
        url: url.to_string(),
        source,
    })?;
    Ok(envelope.data.exact_matches.into_iter().find_map(|hit| {
        let file = hit.file?;
        Some(FingerprintMatch {
            project_id: file.mod_id.to_string(),
            file_id: file.id.to_string(),
        })
    }))
}
