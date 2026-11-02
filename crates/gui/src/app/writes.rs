//! Keeps saves for one key in order. Writes run off the UI thread, and two of them for the same
//! key can finish in either order. So a key has at most one write in flight: a save that arrives
//! during it only replaces the snapshot waiting behind it, and the newest snapshot is written once
//! the running write completes. The stored state therefore always ends on the latest edit.

use std::collections::{HashMap, HashSet};
use std::hash::Hash;

/// The writes in flight and the snapshots waiting behind them, by key.
#[derive(Debug)]
pub struct KeyedWriter<K, V> {
    in_flight: HashSet<K>,
    waiting: HashMap<K, V>,
}

impl<K: Eq + Hash + Clone, V> Default for KeyedWriter<K, V> {
    fn default() -> Self {
        Self {
            in_flight: HashSet::new(),
            waiting: HashMap::new(),
        }
    }
}

impl<K: Eq + Hash + Clone, V> KeyedWriter<K, V> {
    /// Asks to save `value` under `key`. Returns the value to write now, or `None` when a write for
    /// `key` is running, in which case `value` replaces any snapshot waiting for it.
    pub fn request(&mut self, key: K, value: V) -> Option<V> {
        if self.in_flight.contains(&key) {
            self.waiting.insert(key, value);
            None
        } else {
            self.in_flight.insert(key);
            Some(value)
        }
    }

    /// Records that the write for `key` finished. Returns the newest snapshot that arrived during it,
    /// which the caller writes next, or `None` when nothing is waiting and the key is free again.
    pub fn complete(&mut self, key: &K) -> Option<V> {
        match self.waiting.remove(key) {
            Some(value) => Some(value),
            None => {
                self.in_flight.remove(key);
                None
            }
        }
    }
}

#[cfg(test)]
#[path = "../../tests/app/writes.rs"]
mod tests;
