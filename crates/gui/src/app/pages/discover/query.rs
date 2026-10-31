//! What a search asks for, the hits on screen, and the pages kept for a while so
//! going back to a search, or scrolling back through it, does not ask the
//! platform again.

use std::collections::HashMap;
use std::time::{Duration, Instant};

use lodestone_platforms::{ContentType, SearchPage, SearchQuery, SearchResult, SearchSort};

use super::Source;

/// How many hits one request asks for.
pub const PAGE_SIZE: u32 = 20;
/// How long a page stays in the cache. The platforms rate-limit their searches, so a search repeated
/// within this window is answered without a request.
const TTL: Duration = Duration::from_secs(10 * 60);
/// How many pages the cache keeps.
const CAPACITY: usize = 64;

/// The parts of a search that decide its answer. The offset is not one of them: the cache keeps each
/// page of a search beside its offset.
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct Key {
    pub source: Source,
    pub content: ContentType,
    pub text: String,
    pub sort: SearchSort,
    pub category: Option<String>,
    pub loader: Option<String>,
    pub game_version: Option<String>,
}

impl Key {
    /// The query this key asks the platform, at `offset`.
    pub fn query(&self, offset: u32) -> SearchQuery {
        SearchQuery {
            text: self.text.clone(),
            content: self.content,
            sort: self.sort,
            offset,
            limit: PAGE_SIZE,
            category: self.category.clone(),
            loader: self.loader.clone(),
            game_version: self.game_version.clone(),
        }
    }
}

/// A page kept for the cache, with how long its request took.
struct Entry {
    page: SearchPage,
    elapsed_ms: u128,
    stored: Instant,
}

/// The pages of recent searches, by search and offset.
#[derive(Default)]
pub struct Cache {
    entries: HashMap<(Key, u32), Entry>,
}

impl Cache {
    /// The fresh page at `offset` of the search `key`, and how long its request took.
    pub fn get(&self, key: &Key, offset: u32) -> Option<(&SearchPage, u128)> {
        self.get_at(key, offset, Instant::now())
    }

    fn get_at(&self, key: &Key, offset: u32, now: Instant) -> Option<(&SearchPage, u128)> {
        self.entries
            .get(&(key.clone(), offset))
            .filter(|entry| now.saturating_duration_since(entry.stored) < TTL)
            .map(|entry| (&entry.page, entry.elapsed_ms))
    }

    /// Keeps `page`, the answer at `offset` of the search `key`. Stale pages go first, and the oldest one
    /// goes when the cache is full.
    pub fn put(&mut self, key: Key, offset: u32, page: SearchPage, elapsed_ms: u128) {
        self.put_at(key, offset, page, elapsed_ms, Instant::now());
    }

    fn put_at(&mut self, key: Key, offset: u32, page: SearchPage, elapsed_ms: u128, now: Instant) {
        self.entries.retain(|_, entry| now.saturating_duration_since(entry.stored) < TTL);
        let slot = (key, offset);
        if self.entries.len() >= CAPACITY && !self.entries.contains_key(&slot) {
            let oldest = self.entries.iter().min_by_key(|(_, entry)| entry.stored).map(|(slot, _)| slot.clone());
            if let Some(oldest) = oldest {
                self.entries.remove(&oldest);
            }
        }
        self.entries.insert(
            slot,
            Entry {
                page,
                elapsed_ms,
                stored: now,
            },
        );
    }
}

/// The hits on screen for the current search, and what the last request said.
#[derive(Debug, Default)]
pub struct Results {
    pub hits: Vec<SearchResult>,
    /// How many hits match in all, when the platform says; 0 otherwise.
    pub total: u64,
    /// How many hits the last page held. It decides whether another page is worth asking for
    /// when the platform reports no total.
    pub last_page: usize,
    pub loading: bool,
    pub error: Option<String>,
    /// How long the request behind these hits took, and whether the cache answered it.
    pub elapsed_ms: Option<u128>,
    pub cached: bool,
}

impl Results {
    /// Whether the platform has hits past the ones on screen.
    pub fn has_more(&self) -> bool {
        if self.total > 0 {
            (self.hits.len() as u64) < self.total
        } else {
            self.last_page == PAGE_SIZE as usize
        }
    }
}

#[cfg(test)]
#[path = "../../../../tests/app/pages/discover/query.rs"]
mod tests;
