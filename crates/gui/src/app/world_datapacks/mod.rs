//! Datapacks for a world: the dialog that lists the datapacks a world holds, each removable, and the datapacks in the shared
//! folder, each addable. A world is one save folder of an instance, so a change waits until the instance's game is stopped,
//! as a backup and a delete do: the game holds the world open. The dialog is drawn over every page, and the root reads the
//! folders and makes the copies and removals.

mod dialog;

use std::path::{Path, PathBuf};

use iced::Task;
use lodestone_platforms::ContentType;
use tracing::{info, warn};

use super::LodestoneAppMessage;
use super::LodestoneLauncher;
use super::components::alert::Alert;
use super::lang;
use super::pages::instance::files::{self, PackFile};
use super::shared_library::{self, SharedEntry};

pub use dialog::view;

/// The dialog's state: the world it is for, the datapacks the world holds, and the shared datapacks, each with whether the
/// world holds it already.
#[derive(Debug, Clone)]
pub struct Dialog {
    /// The instance the world is in.
    pub instance: String,
    /// The world's save folder, which holds its `datapacks/`.
    pub world: PathBuf,
    /// The world's name, as the game shows it.
    pub name: String,
    /// The datapacks the world holds, sorted by name.
    pub installed: Vec<PackFile>,
    /// The datapacks in the shared folder, each with whether the world holds it already.
    pub shared: Vec<(SharedEntry, bool)>,
}

/// Messages of the datapack dialog: the reads, the user's choices, and the outcome of each change.
#[derive(Debug, Clone)]
pub enum Message {
    /// The folders were read for the dialog of a world, or could not be.
    Read(Result<Box<Dialog>, String>),
    /// Add was pressed for the shared datapack at this index of the dialog's list.
    Add(usize),
    /// Remove was pressed for the datapack with this file name.
    Remove(String),
    /// The user answered the confirmation of a removal.
    RemoveAnswered { file_name: String, confirmed: bool },
    /// A shared datapack was copied into the world, under this name, or could not be.
    Copied(Result<String, String>),
    /// A datapack was moved to the trash, or could not be.
    Removed { file_name: String, result: Result<(), String> },
    /// The dialog was closed.
    Close,
}

/// The dialog for a world: its datapacks, and the datapacks in the shared folder with whether the world has each.
fn build_dialog(shared: &Path, instance: String, world: PathBuf, name: String) -> Dialog {
    let installed = files::scan_datapacks(&world);
    let present_in = world.join("datapacks");
    let shared = shared_library::entries(shared)
        .into_iter()
        .map(|entry| {
            let present = present_in.join(&entry.name).exists();
            (entry, present)
        })
        .collect();
    Dialog {
        instance,
        world,
        name,
        installed,
        shared,
    }
}

/// Reads a world's datapacks and the shared ones, for the dialog, on the blocking pool. The answer comes back as `Read`.
fn read_datapacks(shared: PathBuf, instance: String, world: PathBuf, name: String) -> Task<LodestoneAppMessage> {
    Task::perform(
        async move {
            tokio::task::spawn_blocking(move || Box::new(build_dialog(&shared, instance, world, name)))
                .await
                .map_err(|error| error.to_string())
        },
        |result| LodestoneAppMessage::Datapacks(Message::Read(result)),
    )
}

impl LodestoneLauncher {
    /// Opens the datapack dialog for `world`, a save of `instance` named `name`. The dialog reads the folders first.
    pub(super) fn open_datapacks(&self, instance: String, world: PathBuf, name: String) -> Task<LodestoneAppMessage> {
        read_datapacks(shared_library::folder(&self.paths, ContentType::DataPack), instance, world, name)
    }

    /// Carries out a message of the datapack dialog.
    pub(super) fn datapack_message(&mut self, message: Message) -> Task<LodestoneAppMessage> {
        match message {
            Message::Read(Ok(dialog)) => {
                self.datapacks = Some(*dialog);
                Task::none()
            }
            Message::Read(Err(error)) => {
                warn!("Reading the datapacks failed: {error}");
                self.alerts
                    .show(Alert::error(lang::t_with("instance.datapacks.read_failed", &[("error", error.as_str())])))
                    .discard()
            }
            Message::Close => {
                self.datapacks = None;
                Task::none()
            }
            Message::Add(index) => self.add_datapack(index),
            Message::Copied(Ok(name)) => {
                info!("Added the shared datapack {name} to the world");
                self.refresh_datapacks()
            }
            Message::Copied(Err(error)) => {
                warn!("Adding a datapack failed: {error}");
                self.alerts
                    .show(Alert::error(lang::t_with("instance.datapacks.add.failed", &[("error", error.as_str())])))
                    .discard()
            }
            Message::Remove(file_name) => self.ask_datapack_remove(file_name),
            Message::RemoveAnswered { file_name, confirmed } => self.answer_datapack_remove(file_name, confirmed),
            Message::Removed { file_name, result: Ok(()) } => {
                info!("Moved the datapack {file_name} to the trash");
                Task::batch([self.forget_placed_datapack(&file_name), self.refresh_datapacks()])
            }
            Message::Removed { result: Err(error), .. } => {
                warn!("Removing a datapack failed: {error}");
                self.alerts
                    .show(Alert::error(lang::t_with(
                        "instance.datapacks.remove.failed",
                        &[("error", error.as_str())],
                    )))
                    .discard()
            }
        }
    }

    /// Whether the world's instance is running its game, which holds the world open.
    fn datapacks_running(&self) -> bool {
        self.datapacks.as_ref().is_some_and(|dialog| self.games.is_active(&dialog.instance))
    }

    /// Copies the shared datapack at `index` into the world's `datapacks/` folder.
    fn add_datapack(&mut self, index: usize) -> Task<LodestoneAppMessage> {
        if self.datapacks_running() {
            return self.alerts.show(Alert::warn(lang::t("instance.datapacks.running"))).discard();
        }
        let Some(dialog) = self.datapacks.as_ref() else {
            return Task::none();
        };
        let Some((entry, _)) = dialog.shared.get(index).cloned() else {
            return Task::none();
        };
        let destination = dialog.world.join("datapacks");
        Task::perform(
            async move {
                tokio::task::spawn_blocking(move || shared_library::copy_into(&entry.path, &destination))
                    .await
                    .map_err(|error| error.to_string())?
            },
            |result| LodestoneAppMessage::Datapacks(Message::Copied(result)),
        )
    }

    /// Asks before a datapack goes to the trash. The answer comes back as `RemoveAnswered`.
    fn ask_datapack_remove(&mut self, file_name: String) -> Task<LodestoneAppMessage> {
        if self.datapacks_running() {
            return self.alerts.show(Alert::warn(lang::t("instance.datapacks.running"))).discard();
        }
        let message = lang::t_with("instance.packs.remove.message", &[("name", file_name.as_str())]);
        let answered = file_name.clone();
        self.alerts
            .show(Alert::danger(message).title(lang::t("instance.packs.remove.title")))
            .map(move |confirmed| {
                LodestoneAppMessage::Datapacks(Message::RemoveAnswered {
                    file_name: answered.clone(),
                    confirmed,
                })
            })
    }

    /// Moves the datapack to the trash once the user has said yes. The answer comes back as `Removed`.
    fn answer_datapack_remove(&self, file_name: String, confirmed: bool) -> Task<LodestoneAppMessage> {
        if !confirmed {
            return Task::none();
        }
        let Some(dialog) = self.datapacks.as_ref() else {
            return Task::none();
        };
        let path = dialog.world.join("datapacks").join(&file_name);
        let removed = file_name.clone();
        Task::perform(
            async move {
                tokio::task::spawn_blocking(move || files::move_to_trash(&path))
                    .await
                    .map_err(|error| error.to_string())?
            },
            move |result| {
                LodestoneAppMessage::Datapacks(Message::Removed {
                    file_name: removed.clone(),
                    result,
                })
            },
        )
    }

    /// Reads the dialog's folders again, after a datapack was added or removed.
    fn refresh_datapacks(&self) -> Task<LodestoneAppMessage> {
        let Some(dialog) = self.datapacks.as_ref() else {
            return Task::none();
        };
        read_datapacks(
            shared_library::folder(&self.paths, ContentType::DataPack),
            dialog.instance.clone(),
            dialog.world.clone(),
            dialog.name.clone(),
        )
    }

    /// Forgets the record of a datapack the page placed into the world, once it went to the trash.
    fn forget_placed_datapack(&self, file_name: &str) -> Task<LodestoneAppMessage> {
        let (Some(db), Some(dialog)) = (self.db.clone(), self.datapacks.as_ref()) else {
            return Task::none();
        };
        let Some(scope) = dialog.world.file_name().map(|name| name.to_string_lossy().into_owned()) else {
            return Task::none();
        };
        let (instance, file_name) = (dialog.instance.clone(), file_name.to_string());
        Task::future(async move {
            if let Err(error) = db
                .forget_added(&instance, &scope, shared_library::folder_name(ContentType::DataPack), &file_name)
                .await
            {
                warn!("forgetting the placed datapack failed: {error}");
            }
        })
        .discard()
    }
}

#[cfg(test)]
#[path = "../../../tests/app/world_datapacks/mod.rs"]
mod tests;
