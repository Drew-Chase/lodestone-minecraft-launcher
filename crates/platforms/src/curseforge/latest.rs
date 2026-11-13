//! The newest files of many CurseForge projects at once, for an update check. Each project's record lists the latest file
//! for every Minecraft version, loader and release channel (`latestFilesIndexes`), so one batched read of the projects
//! names the candidates, and one batched read of those files gives their records. An update check of any number of mods
//! therefore takes two requests per batch rather than one per mod.

use std::collections::{HashMap, HashSet};

use serde::Deserialize;

use super::{CfFile, LOADERS, available, file_to_content_version, request};
use crate::error::PlatformError;
use crate::http::post_json;
use crate::meta::{ContentType, ContentVersion};

/// The most ids one batched request carries.
const BATCH: usize = 100;

#[derive(Deserialize)]
struct CfModList {
    data: Vec<CfIndexedMod>,
}

/// The part of a project record the check reads: its id, and the latest file of each version, loader and channel.
#[derive(Deserialize)]
struct CfIndexedMod {
    id: i64,
    #[serde(rename = "latestFilesIndexes", default)]
    latest_files_indexes: Vec<CfFileIndex>,
}

#[derive(Deserialize)]
struct CfFileIndex {
    #[serde(rename = "gameVersion", default)]
    game_version: Option<String>,
    #[serde(rename = "fileId")]
    file_id: i64,
    /// CurseForge's loader id, as `LOADERS` lists them. A file with no loader tag has none.
    #[serde(rename = "modLoader", default)]
    mod_loader: Option<u32>,
}

#[derive(Deserialize)]
struct CfFileList {
    data: Vec<CfFile>,
}

/// The newest files of each project in `project_ids` that run on `game_version` and `loader`, newest first, keyed by
/// project id. A project CurseForge answered for is in the map, with no files when none fits, so the caller can tell a
/// project with no update from one it could not check. Reached through `Platform::<CurseForge>::latest_files`.
pub(crate) async fn latest_files(
    credential: Option<&str>,
    project_ids: &[String],
    game_version: &str,
    loader: &str,
) -> Result<HashMap<String, Vec<ContentVersion>>, PlatformError> {
    let ids: Vec<i64> = project_ids
        .iter()
        .filter_map(|id| id.parse().ok())
        .collect::<HashSet<i64>>()
        .into_iter()
        .collect();
    let loader_id = LOADERS.iter().find(|(name, _)| *name == loader).map(|(_, id)| *id);
    let mut found: HashMap<String, Vec<ContentVersion>> = HashMap::new();
    // The candidate files, with the project each belongs to.
    let mut candidates: HashMap<i64, String> = HashMap::new();
    for batch in ids.chunks(BATCH) {
        let (url, client) = request(credential, "/mods")?;
        let response = post_json(&client, &url, &serde_json::json!({ "modIds": batch })).await?;
        let list: CfModList = serde_json::from_value(response).map_err(|source| PlatformError::Json { url, source })?;
        for project in list.data {
            let project_id = project.id.to_string();
            for index in &project.latest_files_indexes {
                if index.game_version.as_deref() == Some(game_version) && loader_id.is_some() && index.mod_loader == loader_id {
                    candidates.insert(index.file_id, project_id.clone());
                }
            }
            found.entry(project_id).or_default();
        }
    }
    let file_ids: Vec<i64> = candidates.keys().copied().collect();
    for batch in file_ids.chunks(BATCH) {
        let (url, client) = request(credential, "/mods/files")?;
        let response = post_json(&client, &url, &serde_json::json!({ "fileIds": batch })).await?;
        let list: CfFileList = serde_json::from_value(response).map_err(|source| PlatformError::Json { url, source })?;
        for file in available(list.data) {
            let Some(project_id) = candidates.get(&file.id).cloned() else {
                continue;
            };
            let release = file_to_content_version(&project_id, file, ContentType::Mod);
            found.entry(project_id).or_default().push(release);
        }
    }
    for releases in found.values_mut() {
        sort_newest_first(releases);
    }
    Ok(found)
}

/// Orders releases newest first, as a file listing is: by date, and by id where dates tie or are missing. CurseForge's dates
/// are ISO 8601 in UTC, so they order as text.
pub(super) fn sort_newest_first(releases: &mut [ContentVersion]) {
    releases.sort_by(|a, b| {
        let id = |release: &ContentVersion| release.id.parse::<i64>().unwrap_or(0);
        b.date.cmp(&a.date).then_with(|| id(b).cmp(&id(a)))
    });
}
