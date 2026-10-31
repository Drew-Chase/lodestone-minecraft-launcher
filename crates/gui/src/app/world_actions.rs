//! The Worlds tab's actions the root carries out: playing a world, opening its folder, copying its seed,
//! backing it up, importing a zipped world, and moving it to the trash once the user has said so. The
//! tab asks for them. The alerts, the file dialog and the game belong to the root.

use std::path::{Path, PathBuf};

use chrono::Local;
use iced::Task;
use tracing::{info, warn};

use super::instances::reveal_folder;
use super::screenshot_actions::file_name;
use super::{LodestoneAppMessage, LodestoneLauncher};
use crate::app::components::alert::Alert;
use crate::app::lang;
use crate::app::pages::instance::Message as InstanceMessage;
use crate::app::pages::instance::files;
use crate::app::pages::instance::world_menu::Action;
use crate::app::pages::instance::worlds;

impl LodestoneLauncher {
    /// Carries out the world menu row or button the Worlds tab picked. A separator carries no action.
    pub(super) fn world_action(&mut self, path: PathBuf, action: Option<Action>) -> Task<LodestoneAppMessage> {
        match action {
            Some(Action::Play) => self.play_world(&path),
            Some(Action::Reveal) => reveal_folder(path),
            Some(Action::CopySeed) => self.copy_seed(&path),
            Some(Action::BackUp) => self.back_up_world(path),
            Some(Action::Delete) => self.ask_world_delete(path),
            None => Task::none(),
        }
    }

    /// Starts the instance into the world at `path`. A version that cannot quick play into a world starts
    /// the instance alone.
    fn play_world(&mut self, path: &Path) -> Task<LodestoneAppMessage> {
        let Some(instance) = self.instance_page.instance() else {
            return Task::none();
        };
        let id = instance.id.clone();
        match path.file_name() {
            Some(folder) => self.launch_world(&id, &folder.to_string_lossy()),
            None => Task::none(),
        }
    }

    /// Starts the instance so the game creates a new world on its first load.
    pub(super) fn new_world(&mut self) -> Task<LodestoneAppMessage> {
        match self.instance_page.instance().map(|instance| instance.id.clone()) {
            Some(id) => self.launch_new_game(&id),
            None => Task::none(),
        }
    }

    /// Copies the world's seed. A world without a seed has nothing to copy.
    fn copy_seed(&self, path: &Path) -> Task<LodestoneAppMessage> {
        match self.instance_page.world(path).and_then(|world| world.seed) {
            Some(seed) => iced::clipboard::write(seed.to_string()),
            None => Task::none(),
        }
    }

    /// Zips the world into the instance's `backups/` folder. A game running on the instance may be saving
    /// the world, so the backup waits until it stops. The outcome returns as `BackedUp`.
    fn back_up_world(&mut self, path: PathBuf) -> Task<LodestoneAppMessage> {
        let Some(instance) = self.instance_page.instance() else {
            return Task::none();
        };
        if self.games.is_active(&instance.id) {
            return self.alerts.show(Alert::warn(lang::t("instance.worlds.backup.running"))).discard();
        }
        let backups = self.paths.instance(&instance.directory_name).join("backups");
        Task::perform(back_up_archive(path, backups), |result| {
            LodestoneAppMessage::Instance(InstanceMessage::Worlds(worlds::Message::BackedUp(result)))
        })
    }

    /// Reports a backup that was written, or could not be.
    pub(super) fn world_backed_up(&mut self, result: Result<PathBuf, String>) -> Task<LodestoneAppMessage> {
        match result {
            Ok(archive) => {
                info!("Backed up a world to {}", archive.display());
                let name = file_name(&archive);
                self.alerts
                    .show(Alert::info(lang::t_with("instance.worlds.backup.done", &[("name", name.as_str())])))
                    .discard()
            }
            Err(error) => {
                warn!("Backing up a world failed: {error}");
                self.alerts
                    .show(Alert::error(lang::t_with("instance.worlds.backup.failed", &[("error", error.as_str())])))
                    .discard()
            }
        }
    }

    /// Asks before the world goes to the trash. The answer returns as `DeleteAnswered`. A game running on
    /// the instance may have the world open, so the delete waits until it stops.
    fn ask_world_delete(&mut self, path: PathBuf) -> Task<LodestoneAppMessage> {
        let Some(instance) = self.instance_page.instance() else {
            return Task::none();
        };
        if self.games.is_active(&instance.id) {
            return self.alerts.show(Alert::warn(lang::t("instance.worlds.delete.running"))).discard();
        }
        let name = self
            .instance_page
            .world(&path)
            .map_or_else(|| file_name(&path), |world| world.name.clone());
        let message = lang::t_with("instance.worlds.delete.message", &[("name", name.as_str())]);
        let answered = path.clone();
        self.alerts
            .show(Alert::danger(message).title(lang::t("instance.worlds.delete.title")))
            .map(move |confirmed| {
                LodestoneAppMessage::Instance(InstanceMessage::Worlds(worlds::Message::DeleteAnswered {
                    path: answered.clone(),
                    confirmed,
                }))
            })
    }

    /// Moves the world to the trash once the user has said yes. The result returns as `Deleted`.
    pub(super) fn answer_world_delete(&mut self, path: PathBuf, confirmed: bool) -> Task<LodestoneAppMessage> {
        if !confirmed {
            return Task::none();
        }
        let deleted = path.clone();
        Task::perform(
            async move {
                tokio::task::spawn_blocking(move || files::move_to_trash(&path))
                    .await
                    .map_err(|error| error.to_string())?
            },
            move |result| {
                LodestoneAppMessage::Instance(InstanceMessage::Worlds(worlds::Message::Deleted {
                    path: deleted.clone(),
                    result,
                }))
            },
        )
    }

    /// Drops a world that went to the trash from the tab. A failure is reported.
    pub(super) fn world_deleted(&mut self, path: PathBuf, result: Result<(), String>) -> Task<LodestoneAppMessage> {
        match result {
            Ok(()) => {
                info!("Moved world {} to the trash", path.display());
                self.instance_page.remove_world(&path);
                Task::none()
            }
            Err(error) => {
                warn!("Moving world {} to the trash failed: {error}", path.display());
                let name = file_name(&path);
                self.alerts
                    .show(Alert::error(lang::t_with(
                        "instance.worlds.delete.failed",
                        &[("name", name.as_str()), ("error", error.as_str())],
                    )))
                    .discard()
            }
        }
    }

    /// Asks for a zipped world, and extracts it into the instance's `saves/`. Cancelling the dialog does
    /// nothing. The outcome returns as `Imported`.
    pub(super) fn import_world(&mut self) -> Task<LodestoneAppMessage> {
        let Some(instance) = self.instance_page.instance() else {
            return Task::none();
        };
        let saves = self.paths.instance(&instance.directory_name).join("saves");
        Task::perform(pick_archive(), |picked| picked).then(move |picked| match picked {
            Some(archive) => Task::perform(import_archive(archive, saves.clone()), |result| {
                LodestoneAppMessage::Instance(InstanceMessage::Worlds(worlds::Message::Imported(result)))
            }),
            None => Task::none(),
        })
    }

    /// Reports a world that was imported, and reads the saves again so it is listed. A failure is reported.
    pub(super) fn world_imported(&mut self, result: Result<String, String>) -> Task<LodestoneAppMessage> {
        match result {
            Ok(folder) => {
                info!("Imported the world {folder}");
                self.instance_page.rescan_worlds().map(LodestoneAppMessage::Instance)
            }
            Err(error) => {
                warn!("Importing a world failed: {error}");
                self.alerts
                    .show(Alert::error(lang::t_with("instance.worlds.import_failed", &[("error", error.as_str())])))
                    .discard()
            }
        }
    }
}

/// The zipped world the user picks, or `None` when they cancel.
async fn pick_archive() -> Option<PathBuf> {
    let file = rfd::AsyncFileDialog::new()
        .set_title(lang::t("instance.worlds.pick.title"))
        .add_filter(lang::t("instance.worlds.pick.filter"), &["zip"])
        .pick_file()
        .await?;
    Some(file.path().to_path_buf())
}

/// Extracts the zip at `archive` into `saves` on the blocking pool.
async fn import_archive(archive: PathBuf, saves: PathBuf) -> Result<String, String> {
    tokio::task::spawn_blocking(move || files::world_archive::import(&archive, &saves))
        .await
        .map_err(|error| error.to_string())?
}

/// Zips the world at `world` into `backups` on the blocking pool.
async fn back_up_archive(world: PathBuf, backups: PathBuf) -> Result<PathBuf, String> {
    tokio::task::spawn_blocking(move || files::world_archive::back_up(&world, &backups, Local::now()))
        .await
        .map_err(|error| error.to_string())?
}
