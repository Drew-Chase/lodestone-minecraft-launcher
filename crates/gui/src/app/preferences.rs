//! Launcher preferences: the choices the user makes that outlive a session.
//!
//! Kept as JSON at `<config_dir>/lodestone/preferences.json`, next to the
//! credential file. `boot` loads it before the first view, so the saved language
//! is already active on the first frame.

use std::path::{Path, PathBuf};

use serde::{Deserialize, Serialize};

/// The saved choices. A field that is missing from the file takes its default.
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct Preferences {
    /// The language's catalogue code, such as `en_us`. `None` means the default.
    pub language: Option<String>,
}

impl Preferences {
    /// Loads the saved preferences. A missing, unreadable, or corrupt file yields
    /// the defaults, so a broken file never blocks the launcher.
    pub fn load() -> Self {
        Self::load_from(&preferences_path())
    }

    /// Persists the preferences, creating the directory on first write.
    pub fn store(&self) -> Result<(), String> {
        Self::store_to(&preferences_path(), self)
    }

    /// Loads from an explicit path (the entry point for tests).
    pub fn load_from(path: &Path) -> Self {
        let Ok(contents) = std::fs::read_to_string(path) else {
            return Self::default();
        };
        match serde_json::from_str(&contents) {
            Ok(preferences) => preferences,
            Err(error) => {
                tracing::warn!("preferences file is corrupt, ignoring it: {error}");
                Self::default()
            }
        }
    }

    /// Persists to an explicit path (the entry point for tests).
    pub fn store_to(path: &Path, preferences: &Self) -> Result<(), String> {
        if let Some(parent) = path.parent() {
            std::fs::create_dir_all(parent).map_err(|error| format!("creating {}: {error}", parent.display()))?;
        }
        let json = serde_json::to_string_pretty(preferences).map_err(|error| format!("serializing preferences: {error}"))?;
        std::fs::write(path, json).map_err(|error| format!("writing {}: {error}", path.display()))
    }
}

/// `<config_dir>/lodestone/preferences.json`.
fn preferences_path() -> PathBuf {
    let mut path = dirs::config_dir().unwrap_or_else(|| PathBuf::from("."));
    path.push("lodestone");
    path.push("preferences.json");
    path
}

#[cfg(test)]
#[path = "../../tests/app/preferences.rs"]
mod tests;
