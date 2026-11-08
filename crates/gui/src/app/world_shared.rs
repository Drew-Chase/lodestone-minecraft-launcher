//! The shared worlds the Worlds tab adds to an instance: reading the shared `worlds/` folder for its dialog, extracting a
//! download into the instance's `saves/`, and moving a download to the trash once the user has said so. The page keeps what
//! its dialog shows; these are the reads and the changes on disk. A world the Discover page places goes in the same way.

use std::path::{Path, PathBuf};

use iced::Task;
use lodestone_platforms::ContentType;
use tracing::{info, warn};

use super::pages::instance::Message as InstanceMessage;
use super::pages::instance::files::{self, world_archive};
use super::pages::instance::worlds::Message as WorldsMessage;
use super::shared_library::{self, SharedEntry};
use super::world_actions::WorldMessage;
use super::{LodestoneAppMessage, LodestoneLauncher};
use crate::app::components::alert::Alert;
use crate::app::lang;

impl LodestoneLauncher {
    /// Reads the shared worlds for the dialog: each downloaded world, and whether the instance has a save of its name. The
    /// answer comes back on the Worlds tab as `SharedLoaded`.
    pub(super) fn read_shared_worlds(&self, instance: String) -> Task<LodestoneAppMessage> {
        let Some(saves) = self.instance_saves(&instance) else {
            return Task::none();
        };
        let shared = shared_library::folder(&self.paths, ContentType::World);
        Task::perform(
            async move {
                tokio::task::spawn_blocking(move || {
                    let folders = save_folders(&saves);
                    let worlds = shared_library::entries(&shared).into_iter().filter(is_world_archive).collect::<Vec<_>>();
                    mark_present(worlds, &folders)
                })
                .await
                .unwrap_or_default()
            },
            |entries| LodestoneAppMessage::Instance(InstanceMessage::Worlds(WorldsMessage::SharedLoaded { entries })),
        )
    }

    /// Extracts the shared world the dialog lists at `index` into the instance's `saves/`, and records where it came from,
    /// as a pack does, so Discover shows it as added. The answer comes back as `SharedAdded`.
    pub(super) fn add_shared_world(&self, instance: String, index: usize) -> Task<LodestoneAppMessage> {
        let (Some(entry), Some(saves)) = (self.shared_entry(index), self.instance_saves(&instance)) else {
            return Task::none();
        };
        let download = entry.download.clone();
        let db = self.db.clone();
        let recorded = instance.clone();
        Task::perform(
            async move {
                let name = tokio::task::spawn_blocking(move || world_archive::import(&entry.path, &saves))
                    .await
                    .map_err(|error| error.to_string())??;
                if let (Some(db), Some(download)) = (db, download)
                    && let Err(error) = db
                        .record_shared_origin(&recorded, shared_library::folder_name(ContentType::World), &download, &name)
                        .await
                {
                    warn!("recording where the shared world came from failed: {error}");
                }
                Ok(name)
            },
            move |result| LodestoneAppMessage::World(WorldMessage::SharedAdded { instance, result }),
        )
    }

    /// Reports a shared world that was extracted, or could not be. The instance's worlds are read again, and the dialog,
    /// while it is open for that instance, reads the shared folder again so the world shows as added.
    pub(super) fn shared_world_added(&mut self, instance: String, result: Result<String, String>) -> Task<LodestoneAppMessage> {
        match result {
            Ok(folder) => {
                info!("Added the shared world {folder} to the instance");
                let rescan = self.rescan_worlds_of(&instance);
                let reread = if self.dialog_open_for(&instance) {
                    self.read_shared_worlds(instance)
                } else {
                    Task::none()
                };
                Task::batch([rescan, reread])
            }
            Err(error) => {
                warn!("Adding a shared world failed: {error}");
                self.alerts
                    .show(Alert::error(lang::t_with("instance.worlds.import_failed", &[("error", error.as_str())])))
                    .discard()
            }
        }
    }

    /// Asks before the shared world the dialog lists at `index` goes to the trash. The answer comes back as
    /// `SharedDeleteAnswered`.
    pub(super) fn ask_shared_world_delete(&mut self, instance: String, index: usize) -> Task<LodestoneAppMessage> {
        let Some(entry) = self.shared_entry(index) else {
            return Task::none();
        };
        let name = shared_library::world_name(&entry);
        let message = lang::t_with("instance.worlds.shared.delete.message", &[("name", name.as_str())]);
        self.alerts
            .show(Alert::danger(message).title(lang::t("instance.worlds.shared.delete.title")))
            .map(move |confirmed| {
                LodestoneAppMessage::World(WorldMessage::SharedDeleteAnswered {
                    instance: instance.clone(),
                    entry: entry.clone(),
                    confirmed,
                })
            })
    }

    /// Moves the shared world to the trash once the user has said yes. A download's folder goes with it, as a pack's does.
    /// The answer comes back as `SharedDeleted`.
    pub(super) fn answer_shared_world_delete(&self, instance: String, entry: SharedEntry, confirmed: bool) -> Task<LodestoneAppMessage> {
        if !confirmed {
            return Task::none();
        }
        let target = shared_library::delete_target(&entry);
        let deleted = entry.clone();
        Task::perform(
            async move {
                tokio::task::spawn_blocking(move || files::move_to_trash(&target))
                    .await
                    .map_err(|error| error.to_string())?
            },
            move |result| {
                LodestoneAppMessage::World(WorldMessage::SharedDeleted {
                    instance: instance.clone(),
                    entry: deleted.clone(),
                    result,
                })
            },
        )
    }

    /// Forgets the record of a shared download that went to the trash, and reads the dialog again while it shows this
    /// instance, so the world no longer lists. A failure is reported.
    pub(super) fn shared_world_deleted(&mut self, instance: String, entry: SharedEntry, result: Result<(), String>) -> Task<LodestoneAppMessage> {
        match result {
            Ok(()) => {
                info!("Moved the shared world {} to the trash", entry.name);
                let forget = match entry.download {
                    Some(download) => self.forget_shared_download(ContentType::World, download),
                    None => Task::none(),
                };
                let refresh = if self.dialog_open_for(&instance) {
                    self.read_shared_worlds(instance)
                } else {
                    Task::none()
                };
                Task::batch([forget, refresh])
            }
            Err(error) => {
                warn!("Deleting a shared world failed: {error}");
                self.alerts
                    .show(Alert::error(lang::t_with(
                        "instance.worlds.shared.delete.failed",
                        &[("error", error.as_str())],
                    )))
                    .discard()
            }
        }
    }

    /// The instance's `saves/` folder, for the instance `instance` names by its id.
    fn instance_saves(&self, instance: &str) -> Option<PathBuf> {
        let row = self.library.instance(instance)?;
        Some(self.paths.instance(&row.directory_name).join("saves"))
    }

    /// The shared world the Worlds tab's dialog lists at `index`.
    fn shared_entry(&self, index: usize) -> Option<SharedEntry> {
        self.instance_page
            .worlds()
            .dialog()
            .and_then(|dialog| dialog.entries.get(index))
            .map(|(entry, _)| entry.clone())
    }

    /// Whether the dialog that adds shared worlds is open on the instance `instance` names.
    fn dialog_open_for(&self, instance: &str) -> bool {
        self.instance_page.instance().is_some_and(|shown| shown.id == instance) && self.instance_page.worlds().dialog().is_some()
    }
}

/// Whether a shared entry is a world's zip, which an import reads. A folder, or any other file, is not a world.
fn is_world_archive(entry: &SharedEntry) -> bool {
    !entry.is_dir && entry.path.extension().is_some_and(|extension| extension.eq_ignore_ascii_case("zip"))
}

/// Pairs each shared world with whether `folders`, the names of the instance's saves, has the save an import would name it.
fn mark_present(worlds: Vec<SharedEntry>, folders: &[String]) -> Vec<(SharedEntry, bool)> {
    worlds
        .into_iter()
        .map(|entry| {
            let present = folders.contains(&world_archive::save_name(&entry.path));
            (entry, present)
        })
        .collect()
}

/// The names of the folders in `saves`, which are the saves the instance has. A folder that cannot be read has none.
fn save_folders(saves: &Path) -> Vec<String> {
    std::fs::read_dir(saves)
        .into_iter()
        .flatten()
        .filter_map(Result::ok)
        .filter(|entry| entry.path().is_dir())
        .map(|entry| entry.file_name().to_string_lossy().into_owned())
        .collect()
}

#[cfg(test)]
#[path = "../../tests/app/world_shared.rs"]
mod tests;
