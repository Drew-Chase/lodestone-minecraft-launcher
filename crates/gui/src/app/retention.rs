//! Keeps the launcher's in-memory pictures and caches from growing without limit. Each entry records when it was last used,
//! and a sweep drops the ones nobody has used for [`IDLE`]. The root runs the sweep on a timer (see `sweep_caches`), so a
//! page the user has left gives its pictures back without anything else happening.

use std::borrow::Borrow;
use std::collections::HashMap;
use std::hash::Hash;
use std::time::{Duration, Instant};

use iced::Task;

use super::components::scene_art;
use super::pages::Page;
use super::{LodestoneAppMessage, LodestoneLauncher};

/// How long an entry may go unused before the sweep drops it.
pub const IDLE: Duration = Duration::from_secs(30);

/// How often the root runs the sweep. An entry is dropped between [`IDLE`] and [`IDLE`] plus this.
pub const SWEEP_EVERY: Duration = Duration::from_secs(5);

/// A map whose entries expire once they have gone unused for [`IDLE`]. Callers pass the time in, so the expiry can be tested
/// without waiting for it.
#[derive(Debug)]
pub struct Retained<K, V> {
    entries: HashMap<K, Entry<V>>,
}

#[derive(Debug)]
struct Entry<V> {
    value: V,
    last_used: Instant,
}

impl<K, V> Default for Retained<K, V> {
    fn default() -> Self {
        Self { entries: HashMap::new() }
    }
}

impl<K: Eq + Hash, V> Retained<K, V> {
    /// Keeps `value` under `key`, used now.
    pub fn insert(&mut self, key: K, value: V, now: Instant) {
        self.entries.insert(key, Entry { value, last_used: now });
    }

    /// The value under `key`. Reading does not count as a use, so a view can read without keeping the entry alive.
    pub fn get<Q: ?Sized + Eq + Hash>(&self, key: &Q) -> Option<&V>
    where
        K: Borrow<Q>,
    {
        self.entries.get(key).map(|entry| &entry.value)
    }

    /// The value under `key`, counted as used now.
    pub fn get_touch<Q: ?Sized + Eq + Hash>(&mut self, key: &Q, now: Instant) -> Option<&V>
    where
        K: Borrow<Q>,
    {
        let entry = self.entries.get_mut(key)?;
        entry.last_used = now;
        Some(&entry.value)
    }

    /// Counts the entry under `key` as used now, if there is one.
    pub fn touch<Q: ?Sized + Eq + Hash>(&mut self, key: &Q, now: Instant)
    where
        K: Borrow<Q>,
    {
        if let Some(entry) = self.entries.get_mut(key) {
            entry.last_used = now;
        }
    }

    /// Whether an entry is kept under `key`.
    pub fn contains<Q: ?Sized + Eq + Hash>(&self, key: &Q) -> bool
    where
        K: Borrow<Q>,
    {
        self.entries.contains_key(key)
    }

    /// Takes the entry under `key` out.
    pub fn remove<Q: ?Sized + Eq + Hash>(&mut self, key: &Q) -> Option<V>
    where
        K: Borrow<Q>,
    {
        self.entries.remove(key).map(|entry| entry.value)
    }

    /// Keeps the entries `keep` accepts, and drops the others, as a plain map would.
    pub fn retain(&mut self, mut keep: impl FnMut(&K) -> bool) {
        self.entries.retain(|key, _| keep(key));
    }

    /// Counts every entry as used now, for an owner whose pictures are all on show.
    pub fn touch_all(&mut self, now: Instant) {
        for entry in self.entries.values_mut() {
            entry.last_used = now;
        }
    }

    /// Every entry, in no particular order.
    pub fn iter(&self) -> impl Iterator<Item = (&K, &V)> {
        self.entries.iter().map(|(key, entry)| (key, &entry.value))
    }

    /// How many entries are kept.
    #[cfg_attr(not(test), allow(dead_code))]
    pub fn len(&self) -> usize {
        self.entries.len()
    }

    /// Whether no entry is kept.
    #[cfg_attr(not(test), allow(dead_code))]
    pub fn is_empty(&self) -> bool {
        self.entries.is_empty()
    }

    /// Drops every entry.
    pub fn clear(&mut self) {
        self.entries.clear();
    }
}

impl<K: Eq + Hash + Clone, V> Retained<K, V> {
    /// Drops every entry that has gone unused for longer than [`IDLE`], and returns their keys, so the owner can forget
    /// what it asked for and ask again when the picture is needed.
    pub fn sweep(&mut self, now: Instant) -> Vec<K> {
        let mut dropped = Vec::new();
        self.entries.retain(|key, entry| {
            let fresh = now.saturating_duration_since(entry.last_used) <= IDLE;
            if !fresh {
                dropped.push(key.clone());
            }
            fresh
        });
        dropped
    }
}

impl LodestoneLauncher {
    /// Runs one sweep over the owners of pictures and caches, then gives the freed memory back. Each owner first counts what
    /// the screen is using as used, so only what is off screen expires. Owners are added here as they gain a sweep.
    pub(super) fn sweep_caches(&mut self) {
        let now = Instant::now();
        let discover_on_show = matches!(self.router.current(), Page::Discover);
        let resource_on_show = matches!(self.router.current(), Page::Resource);
        let news_on_show = matches!(self.router.current(), Page::News);
        let instance_on_show = matches!(self.router.current(), Page::Instance);
        let skins_on_show = matches!(self.router.current(), Page::Skins);
        let evicted = self.discover.sweep_pictures(now, self.window_size, discover_on_show)
            + self.resource.sweep_pictures(now, resource_on_show)
            + self.news.sweep_pictures(now, news_on_show)
            + self.instance_page.sweep_pictures(now, instance_on_show)
            + self.skins.sweep_previews(now, skins_on_show)
            + self.art.sweep(now)
            + scene_art::sweep_generated(now);
        if evicted > 0 {
            release_freed_memory();
        }
    }
}

impl LodestoneLauncher {
    /// Asks again for the pictures of the page now on show that a sweep dropped while the page was away. Pages that hold no
    /// pictures have nothing to ask for.
    pub(super) fn reshow_pictures(&mut self) -> Task<LodestoneAppMessage> {
        match self.router.current() {
            Page::Discover => self.picture_tasks(),
            Page::News => self.news_pictures(),
            Page::Resource => self.resource_pictures(),
            _ => Task::none(),
        }
    }
}

/// Hands memory the allocator has freed back to the operating system. Pictures are decoded on blocking threads, and glibc
/// keeps the pages those threads freed in their own arenas, so dropping a picture alone does not lower the process's resident
/// size. Only glibc has the call; other platforms return freed pages on their own.
pub fn release_freed_memory() {
    #[cfg(all(target_os = "linux", target_env = "gnu"))]
    {
        unsafe extern "C" {
            fn malloc_trim(pad: usize) -> std::ffi::c_int;
        }
        // SAFETY: `malloc_trim` takes no pointers and only returns pages that are already free to the system.
        unsafe {
            malloc_trim(0);
        }
    }
}

#[cfg(test)]
#[path = "../../tests/app/retention.rs"]
mod tests;
