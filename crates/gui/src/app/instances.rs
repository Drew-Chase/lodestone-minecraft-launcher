//! The instance actions the Library menu, the hero and the instance page share: opening the
//! detail page (on its Settings tab when asked), revealing the folder, and deleting the
//! instance. The database and the alerts belong to the root, so these sit beside
//! `new_instance.rs`.

use std::io::ErrorKind;
use std::path::{Path, PathBuf};

use iced::Task;
use tracing::{info, warn};

use super::{LodestoneAppMessage, LodestoneLauncher};
use crate::app::components::alert::Alert;
use crate::app::database::Database;
use crate::app::downloads;
use crate::app::lang;
use crate::app::pages::Page;
use crate::app::pages::instance::{Message as InstanceMessage, Tab};
use crate::app::pages::library::instance_menu::Action;

impl LodestoneLauncher {
    /// Opens an instance's detail page on `tab`. An id that is no longer loaded is ignored.
    pub(super) fn open_instance_page(&mut self, id: &str, tab: Tab) -> Task<LodestoneAppMessage> {
        let Some(instance) = self.library.instance(id) else {
            warn!("Open instance {id} ignored: the instance is no longer loaded");
            return Task::none();
        };
        let folder = self.paths.instance(&instance.directory_name);
        let scan = self.instance_page.show(instance, folder);
        let select = self.instance_page.update(InstanceMessage::TabSelected(tab));
        Task::batch([
            self.navigate(Page::Instance),
            scan.map(LodestoneAppMessage::Instance),
            select.map(LodestoneAppMessage::Instance),
        ])
    }

    /// Hands the reloaded copy of the instance the page shows to the page, so its state and
    /// playtime are current.
    pub(super) fn refresh_instance_page(&mut self) {
        let Some(id) = self.instance_page.instance().map(|instance| instance.id.clone()) else {
            return;
        };
        if let Some(instance) = self.library.instance(&id) {
            self.instance_page.refresh(instance);
        }
    }

    /// Carries out the row of an instance's menu that was picked. A separator carries no action.
    pub(super) fn instance_menu_action(&mut self, id: &str, action: Option<Action>) -> Task<LodestoneAppMessage> {
        match action {
            Some(Action::Play) => self.launch_game(id),
            Some(Action::Edit) => self.open_instance_page(id, Tab::Settings),
            Some(Action::Reveal) => self.reveal_instance(id),
            Some(Action::Delete) => self.ask_instance_delete(id),
            Some(Action::CancelInstall) => match self.downloads.job_for_instance(id) {
                Some(job) => self.downloads_message(downloads::Message::Cancel(job)),
                None => Task::none(),
            },
            None => Task::none(),
        }
    }

    /// Opens the folder of the instance `id` in the file manager.
    pub(super) fn reveal_instance(&self, id: &str) -> Task<LodestoneAppMessage> {
        match self.library.instance(id) {
            Some(instance) => reveal_folder(self.paths.instance(&instance.directory_name)),
            None => Task::none(),
        }
    }

    /// Opens the folder of the instance on the instance page.
    pub(super) fn reveal_current_instance(&self) -> Task<LodestoneAppMessage> {
        match self.instance_page.instance() {
            Some(instance) => reveal_folder(self.paths.instance(&instance.directory_name)),
            None => Task::none(),
        }
    }

    /// Asks before the instance `id` is deleted. The answer returns as `InstanceDeleteAnswered`.
    pub(super) fn ask_instance_delete(&mut self, id: &str) -> Task<LodestoneAppMessage> {
        // An install that has not finished is still writing its folder, so it is cancelled from the downloads.
        if self.library.pending_install(id).is_some() {
            return Task::none();
        }
        if self.games.is_active(id) {
            return self.alerts.show(Alert::warn(lang::t("instance.delete.running"))).discard();
        }
        let Some(instance) = self.library.instance(id) else {
            return Task::none();
        };
        let message = lang::t_with("instance.delete.message", &[("name", instance.name.as_str())]);
        let id = instance.id.clone();
        self.alerts
            .show(Alert::danger(message).title(lang::t("instance.delete.title")))
            .map(move |confirmed| LodestoneAppMessage::InstanceDeleteAnswered {
                instance: id.clone(),
                confirmed,
            })
    }

    /// Asks before the instance on the instance page is deleted.
    pub(super) fn ask_current_instance_delete(&mut self) -> Task<LodestoneAppMessage> {
        match self.instance_page.instance().map(|instance| instance.id.clone()) {
            Some(id) => self.ask_instance_delete(&id),
            None => Task::none(),
        }
    }

    /// Removes the instance once the user has said yes. A no changes nothing.
    pub(super) fn answer_instance_delete(&mut self, id: String, confirmed: bool) -> Task<LodestoneAppMessage> {
        if !confirmed {
            return Task::none();
        }
        let (Some(instance), Some(db)) = (self.library.instance(&id), self.db.clone()) else {
            warn!("Deleting instance {id} skipped: it is no longer loaded or the database is not open");
            return Task::none();
        };
        let folder = self.paths.instance(&instance.directory_name);
        Task::perform(remove_instance(db, folder, id.clone()), move |result| {
            LodestoneAppMessage::InstanceDeleted {
                instance: id.clone(),
                result,
            }
        })
    }

    /// Finishes a delete: the list reloads, and the instance page steps off an instance that is gone.
    pub(super) fn instance_deleted(&mut self, id: String, result: Result<(), String>) -> Task<LodestoneAppMessage> {
        match result {
            Ok(()) => {
                info!("Deleted instance {id}");
                let showing = self.router.current() == Page::Instance && self.instance_page.instance().is_some_and(|instance| instance.id == id);
                if showing {
                    self.router.replace(Page::Home);
                }
                self.load_instances()
            }
            Err(error) => {
                warn!("Deleting instance {id} failed: {error}");
                let name = self.library.instance(&id).map_or_else(|| id.clone(), |instance| instance.name);
                self.alerts
                    .show(Alert::error(lang::t_with(
                        "instance.delete.failed",
                        &[("name", name.as_str()), ("error", error.as_str())],
                    )))
                    .discard()
            }
        }
    }

    /// Carries out a row of the ⋯ menu on the instance page.
    pub(super) fn current_instance_menu(&mut self, action: Option<Action>) -> Task<LodestoneAppMessage> {
        match self.instance_page.instance().map(|instance| instance.id.clone()) {
            Some(id) => self.instance_menu_action(&id, action),
            None => Task::none(),
        }
    }

    /// Opens the page the instance on the instance page was installed from, in the browser.
    pub(super) fn open_current_source(&self) -> Task<LodestoneAppMessage> {
        match self.instance_page.instance().and_then(|instance| instance.source_url.clone()) {
            Some(url) => open_source(url),
            None => Task::none(),
        }
    }

    /// Reports a modpack page that could not be opened.
    pub(super) fn source_opened(&mut self, result: Result<(), String>) -> Task<LodestoneAppMessage> {
        match result {
            Ok(()) => Task::none(),
            Err(error) => {
                warn!("Opening the modpack page failed: {error}");
                self.alerts
                    .show(Alert::error(lang::t_with("instance.open_source.failed", &[("error", error.as_str())])))
                    .discard()
            }
        }
    }

    /// Reports a folder that could not be opened.
    pub(super) fn folder_opened(&mut self, result: Result<(), String>) -> Task<LodestoneAppMessage> {
        match result {
            Ok(()) => Task::none(),
            Err(error) => {
                warn!("Opening the instance folder failed: {error}");
                self.alerts
                    .show(Alert::error(lang::t_with("instance.reveal.failed", &[("error", error.as_str())])))
                    .discard()
            }
        }
    }
}

/// Opens `folder` in the file manager on the blocking pool. A failure returns as `FolderOpened`.
pub(super) fn reveal_folder(folder: PathBuf) -> Task<LodestoneAppMessage> {
    Task::perform(
        async move {
            match tokio::task::spawn_blocking(move || open::that_detached(&folder)).await {
                Ok(opened) => opened.map_err(|error| error.to_string()),
                Err(error) => Err(error.to_string()),
            }
        },
        LodestoneAppMessage::FolderOpened,
    )
}

/// Opens `url` in the browser on the blocking pool. A failure returns as `SourceOpened`.
pub(super) fn open_source(url: String) -> Task<LodestoneAppMessage> {
    Task::perform(
        async move {
            match tokio::task::spawn_blocking(move || open::that_detached(&url)).await {
                Ok(opened) => opened.map_err(|error| error.to_string()),
                Err(error) => Err(error.to_string()),
            }
        },
        LodestoneAppMessage::SourceOpened,
    )
}

/// Removes the instance's folder, then its row. The folder goes first: if a file is locked, the
/// row stays and the delete can be tried again.
async fn remove_instance(db: Database, folder: PathBuf, id: String) -> Result<(), String> {
    let removal = tokio::task::spawn_blocking(move || remove_folder(&folder))
        .await
        .map_err(|error| error.to_string())?;
    removal?;
    db.delete_instance(&id).await
}

/// Removes `folder` and everything in it. A folder that is already gone counts as removed.
fn remove_folder(folder: &Path) -> Result<(), String> {
    match std::fs::remove_dir_all(folder) {
        Ok(()) => Ok(()),
        Err(error) if error.kind() == ErrorKind::NotFound => Ok(()),
        Err(error) => Err(error.to_string()),
    }
}

#[cfg(test)]
#[path = "../../tests/app/instances.rs"]
mod tests;
