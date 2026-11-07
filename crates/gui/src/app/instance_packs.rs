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
    /// The user answered the confirmation of a shared download's deletion.
    SharedDeleteAnswered {
        kind: ContentType,
        entry: shared_library::SharedEntry,
        confirmed: bool,
    },
    /// A shared download was moved to the trash, or could not be.
    SharedDeleted {
        kind: ContentType,
        entry: shared_library::SharedEntry,
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

    /// Carries out a message of the Content tab's packs that the root answers for. The page's own state is kept first. A
    /// reread that was started before the page was shown again is dropped.
    pub(super) fn instance_packs(&mut self, message: PacksMessage) -> Task<LodestoneAppMessage> {
        if let PacksMessage::Rescanned { generation, .. } | PacksMessage::Sources { generation, .. } = &message
            && *generation != self.instance_page.generation()
        {
            return Task::none();
        }
        if let PacksMessage::Sources { result: Err(error), .. } = &message {
            warn!("reading where the packs were taken from failed: {error}");
        }
        self.instance_page.packs_mut().update(&message);
        match message {
            PacksMessage::Query(_)
            | PacksMessage::CloseShared
            | PacksMessage::SharedLoaded { .. }
            | PacksMessage::Rescanned { .. }
            | PacksMessage::Sources { .. } => Task::none(),
            PacksMessage::OpenShared(kind) => self.read_shared_packs(kind),
            PacksMessage::Add { kind, index } => self.add_shared_pack(kind, index),
            PacksMessage::DeleteShared { kind, index } => self.ask_shared_delete(kind, index),
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
                // The copy recorded where the pack came from, so the Discover page reads its records again to show it as added.
                Task::batch([self.reread_packs(), self.read_shared_packs(kind), self.load_added_content()])
            }
            Message::Copied { result: Err(error), .. } => {
                warn!("Adding a shared pack failed: {error}");
                self.alerts
                    .show(Alert::error(lang::t_with("instance.packs.add.failed", &[("error", error.as_str())])))
                    .discard()
            }
            Message::RemoveAnswered { kind, file_name, confirmed } => self.answer_pack_remove(kind, file_name, confirmed),
            Message::SharedDeleteAnswered { kind, entry, confirmed } => self.answer_shared_delete(kind, entry, confirmed),
            Message::SharedDeleted { kind, entry, result: Ok(()) } => {
                info!("Moved the shared download {} to the trash", entry.name);
                let forget = match entry.download {
                    Some(download) => self.forget_shared_download(kind, download),
                    None => Task::none(),
                };
                // The dialog is read again only while it still shows this kind, so a late answer does not reopen it.
                let refresh = if self.instance_page.packs().dialog().is_some_and(|dialog| dialog.kind == kind) {
                    self.read_shared_packs(kind)
                } else {
                    Task::none()
                };
                Task::batch([forget, refresh])
            }
            Message::SharedDeleted { result: Err(error), .. } => {
                warn!("Deleting a shared download failed: {error}");
                self.alerts
                    .show(Alert::error(lang::t_with(
                        "instance.packs.shared.delete.failed",
                        &[("error", error.as_str())],
                    )))
                    .discard()
            }
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

    /// Copies the shared pack the dialog lists at `index` into the instance's folder for its kind. A pack taken from a download
    /// records the platform the download came from, so the Content tab can show it. The answer comes back as `Copied`.
    fn add_shared_pack(&self, kind: ContentType, index: usize) -> Task<LodestoneAppMessage> {
        let Some((entry, _)) = self.instance_page.packs().dialog().and_then(|dialog| dialog.entries.get(index)).cloned() else {
            return Task::none();
        };
        let Some((instance, instance_folder)) = self.shown_instance_folder() else {
            return Task::none();
        };
        let destination = instance_folder.join(shared_library::folder_name(kind));
        let download = entry.download.clone();
        let db = self.db.clone();
        Task::perform(
            async move {
                let name = tokio::task::spawn_blocking(move || shared_library::copy_into(&entry.path, &destination))
                    .await
                    .map_err(|error| error.to_string())??;
                if let (Some(db), Some(download)) = (db, download)
                    && let Err(error) = db
                        .record_shared_origin(&instance, shared_library::folder_name(kind), &download, &name)
                        .await
                {
                    warn!("recording where the shared pack came from failed: {error}");
                }
                Ok(name)
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

    /// Asks before a shared download goes to the trash. The answer comes back as `SharedDeleteAnswered`.
    fn ask_shared_delete(&mut self, kind: ContentType, index: usize) -> Task<LodestoneAppMessage> {
        let Some(entry) = self
            .instance_page
            .packs()
            .dialog()
            .and_then(|dialog| dialog.entries.get(index))
            .map(|(entry, _)| entry.clone())
        else {
            return Task::none();
        };
        let name = files::pack_name(&entry.name);
        let message = lang::t_with("instance.packs.shared.delete.message", &[("name", name.as_str())]);
        self.alerts
            .show(Alert::danger(message).title(lang::t("instance.packs.shared.delete.title")))
            .map(move |confirmed| {
                LodestoneAppMessage::PackActions(Message::SharedDeleteAnswered {
                    kind,
                    entry: entry.clone(),
                    confirmed,
                })
            })
    }

    /// Moves a shared download to the trash once the user has said yes. A file in a download takes its download's folder
    /// with it, so the download goes as a whole. The answer comes back as `SharedDeleted`.
    fn answer_shared_delete(&self, kind: ContentType, entry: shared_library::SharedEntry, confirmed: bool) -> Task<LodestoneAppMessage> {
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
                LodestoneAppMessage::PackActions(Message::SharedDeleted {
                    kind,
                    entry: deleted.clone(),
                    result,
                })
            },
        )
    }

    /// Forgets the record of a download whose folder went to the trash, then reads the records again, so the Discover page
    /// no longer lists it as downloaded. The records are read after the forgetting, so the answer does not list it.
    fn forget_shared_download(&self, kind: ContentType, download: String) -> Task<LodestoneAppMessage> {
        let Some(db) = self.db.clone() else {
            return Task::none();
        };
        let folder = shared_library::folder_name(kind);
        Task::perform(
            async move {
                if let Err(error) = db.forget_shared_content(folder, &download).await {
                    warn!("forgetting the shared download failed: {error}");
                }
                db.shared_content().await
            },
            LodestoneAppMessage::SharedLoaded,
        )
    }

    /// Reads the platforms the shown instance's packs were taken from, as the database holds them. The answer comes back as
    /// `Sources`, with the page's generation, so a read that lands after the page was shown again is dropped.
    pub(super) fn packs_sources(&self, generation: u64) -> Task<LodestoneAppMessage> {
        let (Some(db), Some(instance)) = (self.db.clone(), self.instance_page.instance().map(|instance| instance.id.clone())) else {
            return Task::none();
        };
        Task::perform(async move { db.added_content_of(&instance).await }, move |result| {
            LodestoneAppMessage::Instance(InstanceMessage::Packs(PacksMessage::Sources { generation, result }))
        })
    }

    /// Reads the packs of the instance again, so the lists show what the folders hold now, and reads the platforms they were
    /// taken from. The answers carry the page's generation, so a read that lands after the page was shown again is dropped.
    pub(super) fn reread_packs(&self) -> Task<LodestoneAppMessage> {
        let Some((_, instance_folder)) = self.shown_instance_folder() else {
            return Task::none();
        };
        let generation = self.instance_page.generation();
        let scan = Task::perform(
            async move {
                tokio::task::spawn_blocking(move || files::scan_packs(&instance_folder))
                    .await
                    .unwrap_or_default()
            },
            move |(resource, shader)| {
                LodestoneAppMessage::Instance(InstanceMessage::Packs(PacksMessage::Rescanned {
                    generation,
                    resource,
                    shader,
                }))
            },
        );
        Task::batch([scan, self.packs_sources(generation)])
    }

    /// Forgets the record of a placed pack that went to the trash, so the page no longer shows it as added. The records are read
    /// again after the forgetting, in the same task, so the answer does not list it.
    fn forget_placed_pack(&self, kind: ContentType, file_name: &str) -> Task<LodestoneAppMessage> {
        let (Some(db), Some((instance, _))) = (self.db.clone(), self.shown_instance_folder()) else {
            return Task::none();
        };
        let file_name = file_name.to_string();
        Task::perform(
            async move {
                if let Err(error) = db.forget_added(&instance, "", shared_library::folder_name(kind), &file_name).await {
                    warn!("forgetting the placed pack failed: {error}");
                }
                db.added_content().await
            },
            LodestoneAppMessage::AddedContentLoaded,
        )
    }
}
