//! The Content tab's packs, as the root carries them out: reading the shared folder for the add dialog, copying a shared pack
//! into the instance, moving an installed pack to the trash once the user says so, showing it in the file browser, and
//! opening Discover on the pack kind for the instance. The page keeps what its lists show; these are the reads and the
//! changes on disk.

use std::path::PathBuf;

use iced::Task;
use lodestone_platforms::ContentType;
use tracing::{info, warn};

use super::LodestoneAppMessage;
use super::LodestoneLauncher;
use super::components::alert::Alert;
use super::lang;
use super::pages::Page;
use super::pages::discover::InstanceChoice;
use super::pages::instance::Message as InstanceMessage;
use super::pages::instance::files;
use super::pages::instance::packs::Message as PacksMessage;
use super::shared_library;

/// What the pack actions answer with: the outcome of a copy, or of a removal, or the user's answer to the confirmation.
#[derive(Debug, Clone)]
pub enum Message {
    /// A shared pack was copied into the instance, under the name it has there, or could not be.
    Copied { kind: ContentType, result: Result<String, String> },
    /// The user answered the confirmation of a removal.
    RemoveAnswered { kind: ContentType, file_name: String, confirmed: bool },
    /// A pack was moved to the trash, or could not be.
    Removed {
        kind: ContentType,
        file_name: String,
        result: Result<(), String>,
    },
}

impl LodestoneLauncher {
    /// The instance the Content tab shows, and its folder on disk.
    fn shown_instance_folder(&self) -> Option<(String, PathBuf)> {
        let instance = self.instance_page.instance()?;
        let directory = self.library.instance(&instance.id)?.directory_name;
        Some((instance.id.clone(), self.paths.instance(&directory)))
    }

    /// Carries out a message of the Content tab's packs that the root answers for. The page's own state is kept first.
    pub(super) fn instance_packs(&mut self, message: PacksMessage) -> Task<LodestoneAppMessage> {
        self.instance_page.packs_mut().update(&message);
        match message {
            PacksMessage::Query(_) | PacksMessage::CloseShared | PacksMessage::SharedLoaded { .. } | PacksMessage::Rescanned { .. } => Task::none(),
            PacksMessage::OpenShared(kind) => self.read_shared_packs(kind),
            PacksMessage::Add { kind, index } => self.add_shared_pack(kind, index),
            PacksMessage::Find(kind) => self.find_packs_on_discover(kind),
            PacksMessage::Reveal { kind, file_name } => self.reveal_pack(kind, &file_name),
            PacksMessage::Remove { kind, file_name } => self.ask_pack_remove(kind, file_name),
        }
    }

    /// Carries out the outcome of a copy or a removal, or the answer to the removal's confirmation.
    pub(super) fn pack_actions(&mut self, message: Message) -> Task<LodestoneAppMessage> {
        match message {
            Message::Copied { kind, result: Ok(name) } => {
                info!("Added the shared pack {name} to the instance");
                Task::batch([self.reread_packs(), self.read_shared_packs(kind)])
            }
            Message::Copied { result: Err(error), .. } => {
                warn!("Adding a shared pack failed: {error}");
                self.alerts
                    .show(Alert::error(lang::t_with("instance.packs.add.failed", &[("error", error.as_str())])))
                    .discard()
            }
            Message::RemoveAnswered { kind, file_name, confirmed } => self.answer_pack_remove(kind, file_name, confirmed),
            Message::Removed {
                kind,
                file_name,
                result: Ok(()),
            } => {
                info!("Moved the pack {file_name} to the trash");
                Task::batch([self.reread_packs(), self.forget_placed_pack(kind, &file_name)])
            }
            Message::Removed { result: Err(error), .. } => {
                warn!("Removing a pack failed: {error}");
                self.alerts
                    .show(Alert::error(lang::t_with("instance.packs.remove.failed", &[("error", error.as_str())])))
                    .discard()
            }
        }
    }

    /// Reads the shared folder of a kind of pack for the add dialog, marking each pack the instance already has. The answer
    /// comes back as `SharedLoaded`.
    fn read_shared_packs(&self, kind: ContentType) -> Task<LodestoneAppMessage> {
        let Some((_, instance_folder)) = self.shown_instance_folder() else {
            return Task::none();
        };
        let shared = shared_library::folder(&self.paths, kind);
        let kind_folder = instance_folder.join(shared_library::folder_name(kind));
        Task::perform(
            async move {
                tokio::task::spawn_blocking(move || {
                    shared_library::entries(&shared)
                        .into_iter()
                        .map(|entry| {
                            let present = kind_folder.join(&entry.name).exists();
                            (entry, present)
                        })
                        .collect::<Vec<_>>()
                })
                .await
                .unwrap_or_default()
            },
            move |entries| LodestoneAppMessage::Instance(InstanceMessage::Packs(PacksMessage::SharedLoaded { kind, entries })),
        )
    }

    /// Copies the shared pack the dialog lists at `index` into the instance's folder for its kind. The answer comes back as
    /// `Copied`.
    fn add_shared_pack(&self, kind: ContentType, index: usize) -> Task<LodestoneAppMessage> {
        let Some((entry, _)) = self.instance_page.packs().dialog().and_then(|dialog| dialog.entries.get(index)).cloned() else {
            return Task::none();
        };
        let Some((_, instance_folder)) = self.shown_instance_folder() else {
            return Task::none();
        };
        let destination = instance_folder.join(shared_library::folder_name(kind));
        Task::perform(
            async move {
                tokio::task::spawn_blocking(move || shared_library::copy_into(&entry.path, &destination))
                    .await
                    .map_err(|error| error.to_string())?
            },
            move |result| LodestoneAppMessage::PackActions(Message::Copied { kind, result }),
        )
    }

    /// Opens Discover on the kind of pack, for the instance the Content tab shows. The page returns to the instance.
    fn find_packs_on_discover(&mut self, kind: ContentType) -> Task<LodestoneAppMessage> {
        let Some(instance) = self.instance_page.instance() else {
            return Task::none();
        };
        let choice = InstanceChoice::of(instance);
        self.discover.prepare_content(choice.clone(), kind);
        let opened = self.navigate(Page::Discover);
        // Set after the navigation, which forgets any earlier origin, so the page can return to this instance.
        self.discover.set_origin(Some(choice));
        opened
    }

    /// Shows a pack in the file browser.
    fn reveal_pack(&self, kind: ContentType, file_name: &str) -> Task<LodestoneAppMessage> {
        let Some((_, instance_folder)) = self.shown_instance_folder() else {
            return Task::none();
        };
        super::instances::reveal_folder(instance_folder.join(shared_library::folder_name(kind)).join(file_name))
    }

    /// Asks before a pack goes to the trash. The answer comes back as `RemoveAnswered`.
    fn ask_pack_remove(&mut self, kind: ContentType, file_name: String) -> Task<LodestoneAppMessage> {
        let message = lang::t_with("instance.packs.remove.message", &[("name", file_name.as_str())]);
        let answered = file_name.clone();
        self.alerts
            .show(Alert::danger(message).title(lang::t("instance.packs.remove.title")))
            .map(move |confirmed| {
                LodestoneAppMessage::PackActions(Message::RemoveAnswered {
                    kind,
                    file_name: answered.clone(),
                    confirmed,
                })
            })
    }

    /// Moves the pack to the trash once the user has said yes. The answer comes back as `Removed`.
    fn answer_pack_remove(&self, kind: ContentType, file_name: String, confirmed: bool) -> Task<LodestoneAppMessage> {
        if !confirmed {
            return Task::none();
        }
        let Some((_, instance_folder)) = self.shown_instance_folder() else {
            return Task::none();
        };
        let path = instance_folder.join(shared_library::folder_name(kind)).join(&file_name);
        let removed = file_name.clone();
        Task::perform(
            async move {
                tokio::task::spawn_blocking(move || files::move_to_trash(&path))
                    .await
                    .map_err(|error| error.to_string())?
            },
            move |result| {
                LodestoneAppMessage::PackActions(Message::Removed {
                    kind,
                    file_name: removed.clone(),
                    result,
                })
            },
        )
    }

    /// Reads the packs of the instance again, so the lists show what the folders hold now.
    fn reread_packs(&self) -> Task<LodestoneAppMessage> {
        let Some((_, instance_folder)) = self.shown_instance_folder() else {
            return Task::none();
        };
        Task::perform(
            async move {
                tokio::task::spawn_blocking(move || files::scan_packs(&instance_folder))
                    .await
                    .unwrap_or_default()
            },
            |(resource, shader)| LodestoneAppMessage::Instance(InstanceMessage::Packs(PacksMessage::Rescanned { resource, shader })),
        )
    }

    /// Forgets the record of a placed pack that went to the trash, so the page no longer shows it as added.
    fn forget_placed_pack(&self, kind: ContentType, file_name: &str) -> Task<LodestoneAppMessage> {
        let (Some(db), Some((instance, _))) = (self.db.clone(), self.shown_instance_folder()) else {
            return Task::none();
        };
        let file_name = file_name.to_string();
        Task::future(async move {
            if let Err(error) = db.forget_added(&instance, "", shared_library::folder_name(kind), &file_name).await {
                warn!("forgetting the placed pack failed: {error}");
            }
        })
        .discard()
    }
}
