//! The Worlds actions the root carries out: playing a world, opening its folder, copying its seed, backing it up,
//! restoring a backup as a new save, pinning it, importing a zipped world, and moving it to the trash once the user
//! has said so. The Worlds pages
//! ask for them, and each request names the instance its world belongs to, so an action never reads the instance
//! another page shows. The alerts, the file dialog and the game belong to the root.

use std::path::{Path, PathBuf};

use chrono::Local;
use iced::Task;
use tracing::{info, warn};

use super::instances::reveal_folder;
use super::screenshot_actions::file_name;
use super::world_pins::WorldPins;
use super::{LodestoneAppMessage, LodestoneLauncher};
use crate::app::components::alert::Alert;
use crate::app::lang;
use crate::app::pages::Page;
use crate::app::pages::instance::files;
use crate::app::pages::instance::files::WorldFile;
use crate::app::pages::instance::world_menu::Action;
use crate::app::pages::worlds::{Menu, Message as WorldsMessage, Scan, Source};

/// A world request from a Worlds page, or the answer to one. The instance is named by its id.
#[derive(Debug, Clone)]
pub enum WorldMessage {
    /// A world's button or menu row was picked. A separator sends `None`.
    Menu {
        instance: String,
        path: PathBuf,
        action: Option<Action>,
    },
    /// The instance's New world was pressed: starts the instance, so the game creates a world on its first load.
    NewWorld { instance: String },
    /// The instance's Import was pressed: asks for a zipped world to extract into its `saves/`.
    Import { instance: String },
    /// The user answered the delete alert for the save at `path`. The root trashes it on a yes.
    DeleteAnswered { instance: String, path: PathBuf, confirmed: bool },
    /// The save at `path` went to the trash, or could not.
    Deleted {
        instance: String,
        path: PathBuf,
        result: Result<(), String>,
    },
    /// A backup was written, or could not be.
    BackedUp { instance: String, result: Result<PathBuf, String> },
    /// The Restore button of a backup row was pressed: the zip at `archive`, which holds the save `folder`, is restored
    /// as a new save of `instance`.
    Restore { instance: String, archive: PathBuf, folder: String },
    /// A backup was restored as the save `folder` under the instance's `saves/`, or could not be.
    Restored { instance: String, result: Result<String, String> },
    /// A world was extracted into the instance's `saves/` under this folder name, or could not be.
    Imported { instance: String, result: Result<String, String> },
    /// A pin was written, or could not be. The pin already shows `pinned`.
    PinSaved {
        instance: String,
        folder: String,
        pinned: bool,
        result: Result<(), String>,
    },
}

impl LodestoneLauncher {
    /// Routes a world request to its action, or a world result to its handler.
    pub(super) fn world_message(&mut self, message: WorldMessage) -> Task<LodestoneAppMessage> {
        match message {
            WorldMessage::Menu { instance, path, action } => self.world_action(&instance, path, action),
            WorldMessage::NewWorld { instance } => self.launch_new_game(&instance),
            WorldMessage::Import { instance } => self.import_world(instance),
            WorldMessage::DeleteAnswered { instance, path, confirmed } => self.answer_world_delete(instance, path, confirmed),
            WorldMessage::Deleted { instance, path, result } => self.world_deleted(instance, path, result),
            WorldMessage::BackedUp { instance, result } => self.world_backed_up(instance, result),
            WorldMessage::Restore { instance, archive, folder } => self.restore_backup(instance, archive, folder),
            WorldMessage::Restored { instance, result } => self.world_restored(instance, result),
            WorldMessage::Imported { instance, result } => self.world_imported(instance, result),
            WorldMessage::PinSaved {
                instance,
                folder,
                pinned,
                result,
            } => self.world_pin_saved(instance, folder, pinned, result),
        }
    }

    /// Sends a request built for the instance the instance page shows. Nothing happens when the page shows none.
    pub(super) fn shown_world(&mut self, request: impl FnOnce(String) -> WorldMessage) -> Task<LodestoneAppMessage> {
        match self.instance_page.instance().map(|instance| instance.id.clone()) {
            Some(instance) => self.world_message(request(instance)),
            None => Task::none(),
        }
    }

    /// Carries out the menu row or button a Worlds page picked for the save at `path` of `instance`. A separator
    /// carries no action.
    fn world_action(&mut self, instance: &str, path: PathBuf, action: Option<Action>) -> Task<LodestoneAppMessage> {
        match action {
            Some(Action::Play) => self.play_world(instance, &path),
            Some(Action::Reveal) => reveal_folder(path),
            Some(Action::CopySeed) => self.copy_seed(instance, &path),
            Some(Action::Pin) => self.set_world_pinned(instance, &path, true),
            Some(Action::Unpin) => self.set_world_pinned(instance, &path, false),
            Some(Action::BackUp) => self.back_up_world(instance, path),
            Some(Action::Delete) => self.ask_world_delete(instance, path),
            None => Task::none(),
        }
    }

    /// Starts `instance` into the world at `path`. A version that cannot quick play into a world starts the
    /// instance alone.
    fn play_world(&mut self, instance: &str, path: &Path) -> Task<LodestoneAppMessage> {
        match path.file_name() {
            Some(folder) => self.launch_world(instance, &folder.to_string_lossy()),
            None => Task::none(),
        }
    }

    /// The save at `path` of `instance`, from whichever Worlds page lists it.
    fn world_file(&self, instance: &str, path: &Path) -> Option<&WorldFile> {
        self.worlds.world(instance, path).or_else(|| {
            let shown = self.instance_page.instance()?;
            (shown.id == instance).then(|| self.instance_page.world(path)).flatten()
        })
    }

    /// Copies the seed of the save at `path`. A world without a seed has nothing to copy.
    fn copy_seed(&self, instance: &str, path: &Path) -> Task<LodestoneAppMessage> {
        match self.world_file(instance, path).and_then(|world| world.seed) {
            Some(seed) => iced::clipboard::write(seed.to_string()),
            None => Task::none(),
        }
    }

    /// Pins or unpins the save at `path` of `instance`. The pin shows at once and the database write follows. The
    /// outcome returns as `PinSaved`, which puts the pin back if the write failed.
    fn set_world_pinned(&mut self, instance: &str, path: &Path, pinned: bool) -> Task<LodestoneAppMessage> {
        let Some(folder) = folder_name(path) else {
            return Task::none();
        };
        let Some(db) = self.db.clone() else {
            return Task::none();
        };
        self.world_pins.set(instance, &folder, pinned);
        self.worlds.set_pins(self.world_pins.clone());
        let (write_instance, write_folder) = (instance.to_owned(), folder.clone());
        let (answer_instance, answer_folder) = (instance.to_owned(), folder);
        Task::perform(
            async move { db.set_world_pin(&write_instance, &write_folder, pinned).await },
            move |result| {
                LodestoneAppMessage::World(WorldMessage::PinSaved {
                    instance: answer_instance,
                    folder: answer_folder,
                    pinned,
                    result,
                })
            },
        )
    }

    /// Reports a pin write that failed, and puts the pin back the way it was.
    fn world_pin_saved(&mut self, instance: String, folder: String, pinned: bool, result: Result<(), String>) -> Task<LodestoneAppMessage> {
        match result {
            Ok(()) => Task::none(),
            Err(error) => {
                warn!("Saving the pin of {folder} failed: {error}");
                self.world_pins.set(&instance, &folder, !pinned);
                self.worlds.set_pins(self.world_pins.clone());
                self.alerts
                    .show(Alert::error(lang::t_with("instance.worlds.pin.failed", &[("error", error.as_str())])))
                    .discard()
            }
        }
    }

    /// Zips the save at `path` into the instance's `backups/` folder. A game running on the instance may be saving
    /// the world, so the backup waits until it stops. The outcome returns as `BackedUp`.
    fn back_up_world(&mut self, instance: &str, path: PathBuf) -> Task<LodestoneAppMessage> {
        let Some(row) = self.library.instance(instance) else {
            return Task::none();
        };
        if self.games.is_active(&row.id) {
            return self.alerts.show(Alert::warn(lang::t("instance.worlds.backup.running"))).discard();
        }
        let backups = self.paths.instance(&row.directory_name).join("backups");
        let instance = instance.to_owned();
        Task::perform(back_up_archive(path, backups), move |result| {
            LodestoneAppMessage::World(WorldMessage::BackedUp {
                instance: instance.clone(),
                result,
            })
        })
    }

    /// Reports a backup that was written, or could not be. Either way the lists that show `instance` read again, so
    /// a backed-up world shows its badge.
    fn world_backed_up(&mut self, instance: String, result: Result<PathBuf, String>) -> Task<LodestoneAppMessage> {
        let report = match result {
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
        };
        Task::batch([report, self.rescan_worlds_of(&instance)])
    }

    /// Restores the backup at `archive` as a new save of `instance`, which holds the save `folder`. The outcome returns
    /// as `Restored`.
    fn restore_backup(&mut self, instance: String, archive: PathBuf, folder: String) -> Task<LodestoneAppMessage> {
        let Some(row) = self.library.instance(&instance) else {
            return Task::none();
        };
        let saves = self.paths.instance(&row.directory_name).join("saves");
        Task::perform(restore_archive(archive, saves, folder), move |result| {
            LodestoneAppMessage::World(WorldMessage::Restored { instance, result })
        })
    }

    /// Reports a backup that was restored, or could not be. A restored save is listed with the others, so the lists
    /// that show `instance` read again.
    fn world_restored(&mut self, instance: String, result: Result<String, String>) -> Task<LodestoneAppMessage> {
        match result {
            Ok(folder) => {
                info!("Restored a backup as the save {folder}");
                let report = self
                    .alerts
                    .show(Alert::info(lang::t_with("instance.worlds.restore.done", &[("name", folder.as_str())])))
                    .discard();
                Task::batch([report, self.rescan_worlds_of(&instance)])
            }
            Err(error) => {
                warn!("Restoring a backup failed: {error}");
                self.alerts
                    .show(Alert::error(lang::t_with("instance.worlds.restore.failed", &[("error", error.as_str())])))
                    .discard()
            }
        }
    }

    /// Asks before the save at `path` goes to the trash. The answer returns as `DeleteAnswered`. A game running on
    /// the instance may have the world open, so the delete waits until it stops.
    fn ask_world_delete(&mut self, instance: &str, path: PathBuf) -> Task<LodestoneAppMessage> {
        if self.games.is_active(instance) {
            return self.alerts.show(Alert::warn(lang::t("instance.worlds.delete.running"))).discard();
        }
        let name = self
            .world_file(instance, &path)
            .map_or_else(|| file_name(&path), |world| world.name.clone());
        let message = lang::t_with("instance.worlds.delete.message", &[("name", name.as_str())]);
        let (answered_instance, answered_path) = (instance.to_owned(), path);
        self.alerts
            .show(Alert::danger(message).title(lang::t("instance.worlds.delete.title")))
            .map(move |confirmed| {
                LodestoneAppMessage::World(WorldMessage::DeleteAnswered {
                    instance: answered_instance.clone(),
                    path: answered_path.clone(),
                    confirmed,
                })
            })
    }

    /// Moves the save at `path` to the trash once the user has said yes. The result returns as `Deleted`.
    fn answer_world_delete(&mut self, instance: String, path: PathBuf, confirmed: bool) -> Task<LodestoneAppMessage> {
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
                LodestoneAppMessage::World(WorldMessage::Deleted {
                    instance: instance.clone(),
                    path: deleted.clone(),
                    result,
                })
            },
        )
    }

    /// Drops a world that went to the trash from the lists that show it, and unpins it. A failure is reported.
    fn world_deleted(&mut self, instance: String, path: PathBuf, result: Result<(), String>) -> Task<LodestoneAppMessage> {
        match result {
            Ok(()) => {
                info!("Moved world {} to the trash", path.display());
                self.instance_page.remove_world(&path);
                self.worlds.remove(&instance, &path);
                let pinned = folder_name(&path).is_some_and(|folder| self.world_pins.contains(&instance, &folder));
                if pinned {
                    return self.set_world_pinned(&instance, &path, false);
                }
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

    /// Asks for a zipped world, and extracts it into the `saves/` of `instance`. Cancelling the dialog does nothing.
    /// The outcome returns as `Imported`.
    fn import_world(&mut self, instance: String) -> Task<LodestoneAppMessage> {
        let Some(row) = self.library.instance(&instance) else {
            return Task::none();
        };
        let saves = self.paths.instance(&row.directory_name).join("saves");
        Task::perform(pick_archive(), |picked| picked).then(move |picked| match picked {
            Some(archive) => {
                let instance = instance.clone();
                Task::perform(import_archive(archive, saves.clone()), move |result| {
                    LodestoneAppMessage::World(WorldMessage::Imported {
                        instance: instance.clone(),
                        result,
                    })
                })
            }
            None => Task::none(),
        })
    }

    /// Reports a world that was imported, and reads the saves again so it is listed. A failure is reported.
    fn world_imported(&mut self, instance: String, result: Result<String, String>) -> Task<LodestoneAppMessage> {
        match result {
            Ok(folder) => {
                info!("Imported the world {folder}");
                self.rescan_worlds_of(&instance)
            }
            Err(error) => {
                warn!("Importing a world failed: {error}");
                self.alerts
                    .show(Alert::error(lang::t_with("instance.worlds.import_failed", &[("error", error.as_str())])))
                    .discard()
            }
        }
    }

    /// Reads the saves again on the lists that show `instance`: the instance page while it shows that instance, and
    /// the Worlds page while it is open.
    fn rescan_worlds_of(&mut self, instance: &str) -> Task<LodestoneAppMessage> {
        let mut tasks = Vec::new();
        if self.instance_page.instance().is_some_and(|shown| shown.id == instance) {
            tasks.push(self.instance_page.rescan_worlds().map(LodestoneAppMessage::Instance));
        }
        if self.router.current() == Page::Worlds {
            tasks.push(self.open_worlds());
        }
        Task::batch(tasks)
    }

    /// Opens the Worlds page's list of every instance's saves and backups, read on the blocking pool. The page takes
    /// the pins it lists with, and a scan that a newer one has overtaken is dropped when it lands.
    pub(super) fn open_worlds(&mut self) -> Task<LodestoneAppMessage> {
        let sources: Vec<Source> = self
            .library
            .instances()
            .iter()
            .map(|instance| Source {
                id: instance.id.clone(),
                name: instance.name.clone(),
                mc_version: instance.mc_version.clone(),
                folder: self.paths.instance(&instance.directory_name),
            })
            .collect();
        self.worlds.set_pins(self.world_pins.clone());
        let generation = self.worlds.begin_scan(&sources);
        Task::perform(
            async move { tokio::task::spawn_blocking(move || Scan::read(&sources)).await.unwrap_or_default() },
            move |scan| LodestoneAppMessage::Worlds(WorldsMessage::Scanned { generation, scan }),
        )
    }

    /// Loads the pinned worlds from the database. It runs once the database is open.
    pub(super) fn load_world_pins(&self) -> Task<LodestoneAppMessage> {
        let Some(db) = self.db.clone() else {
            return Task::none();
        };
        Task::perform(async move { db.world_pins().await }, LodestoneAppMessage::WorldPinsLoaded)
    }

    /// Takes the pinned worlds the database holds, and gives the Worlds page its copy.
    pub(super) fn world_pins_loaded(&mut self, rows: Result<Vec<(String, String)>, String>) -> Task<LodestoneAppMessage> {
        match rows {
            Ok(rows) => {
                self.world_pins = WorldPins::from_rows(rows);
                self.worlds.set_pins(self.world_pins.clone());
            }
            Err(error) => warn!("Loading the pinned worlds failed: {error}"),
        }
        Task::none()
    }

    /// Starts the action of the page's menu on `instance`: a new world, or an import into its `saves/`.
    pub(super) fn picked_instance(&mut self, menu: Menu, instance: String) -> Task<LodestoneAppMessage> {
        match menu {
            Menu::New => self.world_message(WorldMessage::NewWorld { instance }),
            Menu::Import => self.world_message(WorldMessage::Import { instance }),
            Menu::Sort => Task::none(),
        }
    }
}

/// The folder name of the save at `path`, which is its last component.
fn folder_name(path: &Path) -> Option<String> {
    path.file_name().map(|name| name.to_string_lossy().into_owned())
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

/// Restores the backup at `archive` as a save of `saves` named `folder` on the blocking pool.
async fn restore_archive(archive: PathBuf, saves: PathBuf, folder: String) -> Result<String, String> {
    tokio::task::spawn_blocking(move || files::world_archive::restore(&archive, &saves, &folder))
        .await
        .map_err(|error| error.to_string())?
}
