//! Clearing the launcher's caches: what it can fetch or work out again. That is the cache folder (Discover pictures, the
//! news feed, staged updates and server pack staging), the faces kept for the accounts, the database's cache section and
//! mod caches (the mod index, the mods' update checks and jar hashes), and what is held in memory (the mod index and the
//! answers to recent platform requests). Settings, sign-ins,
//! instances, worlds, shared downloads and the records of where each mod came from stay.

use std::path::{Path, PathBuf};

use iced::Task;
use tracing::{info, warn};

use crate::app::appearance_cache;
use crate::app::components::alert::Alert;
use crate::app::components::scene_art;
use crate::app::lang;
use crate::app::pages::instance::files::mod_index;
use crate::app::pages::instance::format::format_bytes;
use crate::app::pages::server_pack::Step;
use crate::app::paths;

use super::{LodestoneAppMessage, LodestoneLauncher};

/// The cache folder's folder of staged updates. It is kept while an update is downloading or waits to be installed.
const UPDATES: &str = "updates";
/// The cache folder's folder of servers being built. It is kept while a server pack is being written.
const SERVER_PACK: &str = "server-pack";

impl LodestoneLauncher {
    /// Asks before the caches are cleared. The answer returns as `CacheAnswered`.
    pub(super) fn ask_clear_cache(&mut self) -> Task<LodestoneAppMessage> {
        self.alerts
            .show(
                Alert::warn(lang::t("settings.privacy.clear_cache_body"))
                    .title(lang::t("settings.privacy.clear_cache_title"))
                    .yes_no(),
            )
            .map(LodestoneAppMessage::CacheAnswered)
    }

    /// Clears the caches on a yes. The files go on the blocking pool and the database rows in one transaction, and the
    /// result returns as `CacheCleared` with the bytes freed.
    pub(super) fn cache_answered(&mut self, confirmed: bool) -> Task<LodestoneAppMessage> {
        if !confirmed {
            return Task::none();
        }
        info!("Clearing the caches");
        let mut kept = Vec::new();
        if self.updater.holds_files() {
            kept.push(UPDATES);
        }
        if self.server_pack.step == Step::Exporting {
            kept.push(SERVER_PACK);
        }
        // What is held in memory goes now, so nothing reads it again before the files are gone.
        self.mod_checks.clear();
        mod_index::clear();
        lodestone_platforms::clear_answer_cache();
        // The pictures held in memory go too, so each page asks for them again from the cache folder, which is now empty.
        self.discover.clear_pictures();
        self.resource.clear_pictures();
        self.news.clear_pictures();
        self.instance_page.clear_pictures();
        self.art.clear();
        scene_art::clear_generated();
        let db = self.db.clone();
        Task::perform(
            async move {
                if let Some(db) = db
                    && let Err(error) = db.clear_mod_caches().await
                {
                    warn!("Clearing the mod caches in the database failed: {error}");
                }
                tokio::task::spawn_blocking(move || clear_files(&kept))
                    .await
                    .unwrap_or_else(|error| Err(error.to_string()))
            },
            LodestoneAppMessage::CacheCleared,
        )
    }

    /// Says how much the clear freed, or why it failed.
    pub(super) fn cache_cleared(&mut self, result: Result<u64, String>) -> Task<LodestoneAppMessage> {
        let alert = match result {
            Ok(bytes) => {
                info!("Cleared {bytes} bytes of caches");
                Alert::info(lang::t_with(
                    "settings.privacy.clear_cache_done",
                    &[("size", format_bytes(bytes).as_str())],
                ))
            }
            Err(error) => {
                warn!("Clearing the caches failed: {error}");
                Alert::error(lang::t_with("settings.privacy.clear_cache_failed", &[("error", error.as_str())]))
            }
        };
        self.alerts.show(alert).discard()
    }
}

/// Removes the cache folder's entries, but the ones named in `kept`, and the kept faces, and returns the bytes they held.
/// Every entry is tried; the first failure is returned once all have been.
fn clear_files(kept: &[&str]) -> Result<u64, String> {
    let mut freed = 0;
    let mut failure = None;
    let mut targets: Vec<PathBuf> = Vec::new();
    if let Some(cache) = paths::cache_dir()
        && let Ok(entries) = std::fs::read_dir(&cache)
    {
        targets.extend(
            entries
                .filter_map(Result::ok)
                .filter(|entry| !kept.iter().any(|name| entry.file_name() == *name))
                .map(|entry| entry.path()),
        );
    }
    if let Some(faces) = appearance_cache::root() {
        targets.push(faces);
    }
    for target in targets {
        let size = size_of(&target);
        match remove(&target) {
            Ok(()) => freed += size,
            Err(error) => {
                warn!("Could not remove {}: {error}", target.display());
                failure.get_or_insert(format!("{}: {error}", target.display()));
            }
        }
    }
    match failure {
        Some(error) => Err(error),
        None => Ok(freed),
    }
}

/// Removes a file or a folder. One that is already gone counts as removed. Links are removed, never followed.
fn remove(path: &Path) -> std::io::Result<()> {
    let result = match std::fs::symlink_metadata(path) {
        Ok(metadata) if metadata.is_dir() => std::fs::remove_dir_all(path),
        Ok(_) => std::fs::remove_file(path),
        Err(error) => Err(error),
    };
    match result {
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => Ok(()),
        other => other,
    }
}

/// The bytes under `path`, without following links.
fn size_of(path: &Path) -> u64 {
    let Ok(metadata) = std::fs::symlink_metadata(path) else {
        return 0;
    };
    if !metadata.is_dir() {
        return metadata.len();
    }
    std::fs::read_dir(path)
        .map(|entries| entries.filter_map(Result::ok).map(|entry| size_of(&entry.path())).sum())
        .unwrap_or(0)
}
