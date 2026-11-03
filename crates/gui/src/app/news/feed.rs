//! The launcher news feed: Mojang's posts and the Java Edition patch notes, as piston-mc describes them. The documents
//! are fetched through the launcher's network settings. Each entry is read on its own, so an entry the piston-mc types do
//! not describe (a category they do not name, a missing field) is skipped and counted, rather than failing the whole feed.

use chrono::{DateTime, Local};
use piston_mc::news::NewsReport;
use piston_mc::patch_notes::java::JavaPatchEntry;
use serde::de::DeserializeOwned;
use serde_json::Value;

/// The site Mojang's launcher content is served from. The feed's picture paths are relative to it.
pub const CONTENT_BASE: &str = "https://launchercontent.mojang.com";
/// The news feed: Mojang's posts, newest first.
pub const NEWS_URL: &str = "https://launchercontent.mojang.com/news.json";
/// The Java Edition patch notes, newest first, each with its full HTML body.
pub const PATCHES_URL: &str = "https://launchercontent.mojang.com/javaPatchNotes.json";

/// The feed as the page keeps it.
#[derive(Debug, Clone)]
pub struct Feed {
    /// The posts, in the feed's order.
    pub posts: Vec<NewsReport>,
    /// The Java Edition patch notes, in the feed's order.
    pub patches: Vec<JavaPatchEntry>,
    /// How many entries were left out because they did not read.
    pub skipped: usize,
    /// When the feed was fetched, or when its cached copy was written.
    pub fetched_at: DateTime<Local>,
}

impl Feed {
    /// Reads the two documents, the posts from `news` and the patch notes from `patches`.
    pub fn read(news: &[u8], patches: &[u8], fetched_at: DateTime<Local>) -> Result<Feed, String> {
        let (posts, skipped_posts) = entries::<NewsReport>(news).map_err(|error| format!("the news feed {error}"))?;
        let (patches, skipped_patches) = entries::<JavaPatchEntry>(patches).map_err(|error| format!("the patch notes {error}"))?;
        Ok(Feed {
            posts,
            patches,
            skipped: skipped_posts + skipped_patches,
            fetched_at,
        })
    }
}

/// Reads each entry of a document's `entries` array into `T`. An entry that does not read is counted and left out.
fn entries<T: DeserializeOwned>(bytes: &[u8]) -> Result<(Vec<T>, usize), String> {
    let mut document: Value = serde_json::from_slice(bytes).map_err(|error| format!("is not JSON: {error}"))?;
    let Value::Array(entries) = document.get_mut("entries").map(Value::take).unwrap_or(Value::Null) else {
        return Err("has no list of entries".to_owned());
    };
    let mut read = Vec::with_capacity(entries.len());
    let mut skipped = 0;
    for entry in entries {
        match serde_json::from_value(entry) {
            Ok(item) => read.push(item),
            Err(_) => skipped += 1,
        }
    }
    Ok((read, skipped))
}

/// A picture path from the feed, joined onto the content site. A full URL is kept as it is.
pub fn absolute(url: &str) -> String {
    if url.starts_with("http://") || url.starts_with("https://") {
        url.to_owned()
    } else {
        format!("{CONTENT_BASE}/{}", url.trim_start_matches('/'))
    }
}

/// Downloads both documents, as the bytes the server sent: the news feed, then the patch notes.
pub async fn download() -> Result<(Vec<u8>, Vec<u8>), String> {
    let client = lodestone_net::client();
    let (news, patches) = tokio::join!(get(&client, NEWS_URL), get(&client, PATCHES_URL));
    Ok((news?, patches?))
}

/// One document's bytes, or the reason the server did not send them.
async fn get(client: &reqwest::Client, url: &str) -> Result<Vec<u8>, String> {
    let response = client
        .get(url)
        .send()
        .await
        .and_then(reqwest::Response::error_for_status)
        .map_err(|error| format!("could not fetch {url}: {error}"))?;
    response
        .bytes()
        .await
        .map(|bytes| bytes.to_vec())
        .map_err(|error| format!("could not read {url}: {error}"))
}

#[cfg(test)]
#[path = "../../../tests/app/news/feed.rs"]
mod tests;
