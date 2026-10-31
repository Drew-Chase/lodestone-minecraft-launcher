//! Shared HTTP and download helpers (private).
//!
//! Deliberately closure-free at the call sites of trait methods: closures
//! capturing the surrounding method's reference arguments trip rustc's RPITIT
//! `+ Send` check ("implementation of FnOnce is not general enough") — see
//! `lodestone_loaders`.

use std::collections::HashMap;
use std::path::PathBuf;
use std::sync::{LazyLock, Mutex, MutexGuard, PoisonError};
use std::time::{Duration, Instant};

use reqwest::Client;
use simple_download_utility::{DownloadProgress, download_file};
use tokio::sync::mpsc::Sender;

use crate::error::PlatformError;

/// How many detail calls are in flight at once while a page of results is hydrated. The platforms
/// rate-limit bursts of requests, so a page is hydrated a few calls at a time.
pub(crate) const HYDRATION_CHUNK: usize = 5;
/// How long an answer to a GET is kept. A search, a pack list and a pack's record change over hours,
/// and the platforms rate-limit their APIs, so a repeated read is answered from the cache within this time.
const ANSWER_TTL: Duration = Duration::from_secs(10 * 60);
/// How many answers the cache keeps. The oldest is dropped first.
const ANSWER_CAPACITY: usize = 64;

/// The client the hand-rolled kinds share. ATLauncher sits behind Cloudflare,
/// which turns away requests that carry no User-Agent.
pub(crate) fn client() -> Client {
    Client::builder()
        .user_agent(concat!("lodestone_platforms/", env!("CARGO_PKG_VERSION")))
        .build()
        .unwrap_or_else(|_| Client::new())
}

/// The answers to recent GETs. Every handle shares one cache, as they share the connection pool.
static ANSWERS: LazyLock<Mutex<AnswerCache>> = LazyLock::new(|| Mutex::new(AnswerCache::new(ANSWER_TTL, ANSWER_CAPACITY)));

/// Response bodies by url. A body is kept until it is `ttl` old, and the oldest is dropped when the
/// cache is full.
struct AnswerCache {
    ttl: Duration,
    capacity: usize,
    entries: HashMap<String, Answer>,
}

/// A body the cache keeps, and when it was received.
struct Answer {
    body: String,
    received: Instant,
}

impl AnswerCache {
    fn new(ttl: Duration, capacity: usize) -> Self {
        Self {
            ttl,
            capacity,
            entries: HashMap::new(),
        }
    }

    /// The body kept for `url`, if it is still fresh at `now`.
    fn get(&self, url: &str, now: Instant) -> Option<&str> {
        self.entries
            .get(url)
            .filter(|answer| now.saturating_duration_since(answer.received) < self.ttl)
            .map(|answer| answer.body.as_str())
    }

    /// Keeps `body` for `url`, dropping stale answers first, and the oldest one when the cache is full.
    fn put(&mut self, url: String, body: String, now: Instant) {
        self.entries.retain(|_, answer| now.saturating_duration_since(answer.received) < self.ttl);
        if self.entries.len() >= self.capacity && !self.entries.contains_key(&url) {
            let oldest = self.entries.iter().min_by_key(|(_, answer)| answer.received).map(|(url, _)| url.clone());
            if let Some(oldest) = oldest {
                self.entries.remove(&oldest);
            }
        }
        self.entries.insert(url, Answer { body, received: now });
    }
}

/// The shared cache, locked. A panic while it is locked leaves nothing half-written, so a poisoned lock
/// is used as it is.
fn answers() -> MutexGuard<'static, AnswerCache> {
    ANSWERS.lock().unwrap_or_else(PoisonError::into_inner)
}

/// GETs `url` and deserializes the JSON body, mapping failures onto
/// [`PlatformError`] with the url attached. A body received within the cache's
/// lifetime answers from the cache instead, and only successful answers are kept.
pub(crate) async fn get_json(http: &Client, url: &str) -> Result<serde_json::Value, PlatformError> {
    let cached = answers().get(url, Instant::now()).map(str::to_owned);
    let body = match cached {
        Some(body) => body,
        None => {
            let body = get_text(http, url).await?;
            answers().put(url.to_string(), body.clone(), Instant::now());
            body
        }
    };
    serde_json::from_str(&body).map_err(|source| PlatformError::Json {
        url: url.to_string(),
        source,
    })
}

/// GETs `url` and returns the body of a successful answer.
async fn get_text(http: &Client, url: &str) -> Result<String, PlatformError> {
    let response = http.get(url).send().await.map_err(|source| PlatformError::Network {
        url: url.to_string(),
        source,
    })?;
    let status = response.status();
    if !status.is_success() {
        return Err(PlatformError::UnexpectedResponse {
            url: url.to_string(),
            status: status.as_u16(),
        });
    }
    // Read the body as text and parse from the string so parse failures surface
    // as real `serde_json::Error`s rather than reqwest's opaque wrapper.
    response.text().await.map_err(|source| PlatformError::Network {
        url: url.to_string(),
        source,
    })
}

/// Downloads `(url, destination)` pairs sequentially, creating parent
/// directories as needed, and returns the destination paths in order.
///
/// A progress channel, when given, receives per-byte progress for each file —
/// the same `DownloadProgress` events piston-mc and lodestone_loaders emit, so
/// gui can treat every download uniformly.
pub(crate) async fn download_files(
    files: impl IntoIterator<Item = (String, PathBuf)>,
    progress: Option<&Sender<DownloadProgress>>,
) -> Result<Vec<PathBuf>, PlatformError> {
    let mut downloaded = Vec::new();
    for (url, path) in files {
        if let Some(parent) = path.parent() {
            std::fs::create_dir_all(parent).map_err(|source| PlatformError::Io {
                path: parent.to_path_buf(),
                source,
            })?;
        }
        download_file(&url, &path, progress.cloned())
            .await
            .map_err(|source| PlatformError::Download {
                url: url.clone(),
                source: source.into(),
            })?;
        downloaded.push(path);
    }
    Ok(downloaded)
}

/// Appends a query parameter to `url` with proper percent-encoding.
pub(crate) fn with_query(url: &str, key: &str, value: &str) -> String {
    let separator = if url.contains('?') { '&' } else { '?' };
    format!("{url}{separator}{key}={}", percent_encode(value))
}

/// Percent-encodes everything outside the unreserved set — enough for query
/// values like search terms.
fn percent_encode(value: &str) -> String {
    let mut encoded = String::with_capacity(value.len());
    for byte in value.bytes() {
        match byte {
            b'A'..=b'Z' | b'a'..=b'z' | b'0'..=b'9' | b'-' | b'_' | b'.' | b'~' => {
                encoded.push(byte as char);
            }
            _ => {
                encoded.push('%');
                encoded.push_str(&format!("{byte:02X}"));
            }
        }
    }
    encoded
}

#[cfg(test)]
#[path = "../tests/http.rs"]
mod tests;
