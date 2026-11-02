//! The worlds the player pinned, held in memory for the Worlds pages. The database keeps them in the `world_pins`
//! table: the root loads them at start and writes each change.

use std::collections::{HashMap, HashSet};

/// The pinned saves, by instance id and save folder name.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct WorldPins {
    by_instance: HashMap<String, HashSet<String>>,
}

impl WorldPins {
    /// The pins the database returned, as (instance id, save folder) rows.
    pub fn from_rows(rows: Vec<(String, String)>) -> Self {
        let mut pins = Self::default();
        for (instance, folder) in rows {
            pins.set(&instance, &folder, true);
        }
        pins
    }

    /// Whether the save `folder` of the instance `instance` is pinned.
    pub fn contains(&self, instance: &str, folder: &str) -> bool {
        self.by_instance.get(instance).is_some_and(|folders| folders.contains(folder))
    }

    /// Forgets every pin of the instance `instance`, as it was deleted.
    pub fn forget_instance(&mut self, instance: &str) {
        self.by_instance.remove(instance);
    }

    /// Pins or unpins the save `folder` of the instance `instance`.
    pub fn set(&mut self, instance: &str, folder: &str, pinned: bool) {
        if pinned {
            self.by_instance.entry(instance.to_owned()).or_default().insert(folder.to_owned());
        } else if let Some(folders) = self.by_instance.get_mut(instance) {
            folders.remove(folder);
            let empty = folders.is_empty();
            if empty {
                self.by_instance.remove(instance);
            }
        }
    }
}

#[cfg(test)]
#[path = "../../tests/app/world_pins.rs"]
mod tests;
