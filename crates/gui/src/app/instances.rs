//! The instance actions the Library menu, the hero and the instance page share: opening the
//! detail page (on its Settings tab when asked), revealing the folder, and deleting the
//! instance. The database and the alerts belong to the root, so these sit beside
//! `new_instance.rs`.

use std::io::ErrorKind;
use std::path::{Path, PathBuf};

use chrono::Utc;
use iced::Task;
use lodestone_platforms::ContentType;
use tracing::{info, warn};

use super::reveal::{open_file, reveal_file};
use super::{LodestoneAppMessage, LodestoneLauncher};
use crate::app::components::alert::Alert;
use crate::app::database::Database;
use crate::app::downloads;
use crate::app::install::ART_DIR;
use crate::app::lang;
use crate::app::pages::Page;
use crate::app::pages::discover::{InstanceChoice, Source as DiscoverSource};
use crate::app::pages::instance::settings::{self, Edit};
use crate::app::pages::instance::{Message as InstanceMessage, Tab};
use crate::app::pages::library::instance::{Instance, InstanceIcon, Loader};
use crate::app::pages::library::instance_menu::Action;
use crate::app::pages::library::new_instance::{BannerChoice, IconChoice, banner};
use crate::app::pages::resource::Listing;
use crate::app::pages::worlds::Source;

impl LodestoneLauncher {
    /// Every instance as the Worlds and Servers pages read them: its id and name, its version, and its game folder.
    pub(super) fn instance_sources(&self) -> Vec<Source> {
        self.library
            .instances()
            .iter()
            .map(|instance| Source {
                id: instance.id.clone(),
                name: instance.name.clone(),
                mc_version: instance.mc_version.clone(),
                folder: self.paths.instance(&instance.directory_name),
            })
            .collect()
    }

    /// Opens the Discover page on the Mods tab for the instance on screen, as the Mods tab's Add asks. The page's
    /// loader and Minecraft version become the filters, and the mods are added to this instance.
    pub(super) fn add_mods_from_instance(&mut self) -> Task<LodestoneAppMessage> {
        let Some(instance) = self.instance_page.instance() else {
            return Task::none();
        };
        let choice = InstanceChoice::of(instance);
        self.discover.prepare_mods(choice.clone());
        let opened = self.navigate(Page::Discover);
        // Set after the navigation, which forgets any earlier origin, so the page can return to this instance.
        self.discover.set_origin(Some(choice));
        opened
    }

    /// Opens an instance's detail page on `tab`. An id that is no longer loaded is ignored.
    pub(super) fn open_instance_page(&mut self, id: &str, tab: Tab) -> Task<LodestoneAppMessage> {
        let Some(instance) = self.library.instance(id) else {
            warn!("Open instance {id} ignored: the instance is no longer loaded");
            return Task::none();
        };
        let folder = self.paths.instance(&instance.directory_name);
        let scan = self.instance_page.show(instance, folder, &self.settings_values.instance_defaults);
        let select = self.instance_page.update(InstanceMessage::TabSelected(tab));
        Task::batch([
            self.navigate(Page::Instance),
            scan.map(LodestoneAppMessage::Instance),
            select.map(LodestoneAppMessage::Instance),
            self.icon_recents(),
        ])
    }

    /// Reads the icons used most recently, for the icon editor on the Settings tab.
    fn icon_recents(&self) -> Task<LodestoneAppMessage> {
        let Some(db) = self.db.clone() else {
            return Task::none();
        };
        Task::perform(async move { db.recent_icons().await }, |result| {
            LodestoneAppMessage::Instance(InstanceMessage::Settings(settings::Message::RecentsLoaded(result)))
        })
    }

    /// Queues a fetch of the shown instance's Minecraft version again. It is refused while an instance on that version
    /// runs, because the jar is in use.
    pub(super) fn reinstall_instance(&mut self) -> Task<LodestoneAppMessage> {
        let Some(instance) = self.instance_page.instance() else {
            return Task::none();
        };
        let mc = instance.mc_version.clone();
        let busy = self
            .library
            .instances()
            .iter()
            .any(|other| other.mc_version == mc && self.games.is_active(&other.id));
        if busy {
            return self.alerts.show(Alert::warn(lang::t("instance.settings.reinstall_busy"))).discard();
        }
        let job = self.downloads.enqueue_reinstall(downloads::ReinstallJob {
            name: instance.name.clone(),
            mc_version: mc.clone(),
        });
        Task::perform(crate::app::install::reinstall_version(self.paths.clone(), mc), move |result| {
            LodestoneAppMessage::ReinstallFinished { job, result }
        })
    }

    /// Finishes a reinstall job. The popover shows the outcome, and a failure is also reported in an alert.
    pub(super) fn reinstall_finished(&mut self, job: u64, result: Result<(), String>) -> Task<LodestoneAppMessage> {
        match result {
            Ok(()) => {
                info!("Reinstalled the game files for job {job}");
                self.downloads.finish(job, Ok(()));
                Task::none()
            }
            Err(error) => {
                warn!("Reinstalling the game files failed: {error}");
                self.downloads.finish(job, Err(error.clone()));
                self.alerts
                    .show(Alert::error(lang::t_with(
                        "instance.settings.reinstall_failed",
                        &[("error", error.as_str())],
                    )))
                    .discard()
            }
        }
    }

    /// Hands the reloaded copy of the instance the page shows to the page, so its state and
    /// playtime are current.
    pub(super) fn refresh_instance_page(&mut self) {
        let Some(id) = self.instance_page.instance().map(|instance| instance.id.clone()) else {
            return;
        };
        if let Some(instance) = self.library.instance(&id) {
            self.instance_page.refresh(instance, &self.settings_values.instance_defaults);
        }
    }

    /// Carries out the row of an instance's menu that was picked. A separator carries no action.
    pub(super) fn instance_menu_action(&mut self, id: &str, action: Option<Action>) -> Task<LodestoneAppMessage> {
        match action {
            Some(Action::Play) => self.launch_game(id),
            Some(Action::Edit) => self.open_instance_page(id, Tab::Settings),
            Some(Action::Reveal) => self.reveal_instance(id),
            Some(Action::Export) => self.open_export(id),
            Some(Action::ServerPack) => self.open_server_pack(id),
            Some(Action::Delete) => self.ask_instance_delete(id),
            Some(Action::Lock) => self.lock_instance(id.to_owned()),
            Some(Action::Unlock) => self.ask_unlock(id.to_owned()),
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

    /// Opens the shown instance's `logs/latest.log` in its app, or shows it selected in the file browser. The file is
    /// read by the app that opens it, so a log not yet written fails there, and the failure is reported.
    pub(super) fn latest_log(&self, reveal: bool) -> Task<LodestoneAppMessage> {
        let Some(instance) = self.instance_page.instance() else {
            return Task::none();
        };
        let log = self.paths.instance(&instance.directory_name).join("logs").join("latest.log");
        if reveal { reveal_file(log) } else { open_file(log) }
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
        // An in-place update is writing the instance's folder, so the folder waits for it to end.
        if self.downloads.update_for_instance(id).is_some() {
            return self.alerts.show(Alert::warn(lang::t("pack_update.busy.working"))).discard();
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
        let kept = vec![self.paths.pack_base(&id), self.paths.pack_journal(&id)];
        Task::perform(remove_instance(db, folder, kept, id.clone()), move |result| {
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
                    self.router.replace(Page::Library);
                }
                self.world_pins.forget_instance(&id);
                self.worlds.set_pins(self.world_pins.clone());
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
    /// Opens the modpack the instance came from on its Discover product page, which returns to the instance. An instance
    /// whose platform or project is unknown opens the modpack's web page instead.
    pub(super) fn open_current_source(&mut self) -> Task<LodestoneAppMessage> {
        let Some(instance) = self.instance_page.instance() else {
            return Task::none();
        };
        let choice = InstanceChoice::of(instance);
        let source = instance.source_platform.as_deref().and_then(DiscoverSource::from_key);
        let listing = source.zip(instance.source_project_id.clone()).map(|(source, project_id)| Listing {
            source,
            content: ContentType::Modpack,
            project_id,
            name: instance.name.clone(),
            author: None,
            summary: String::new(),
            downloads: 0,
            icon_url: None,
            web_url: instance.source_url.clone(),
            updated: None,
        });
        let web = instance.source_url.clone();
        match listing {
            Some(listing) => self.open_resource(listing, Some(choice)),
            None => web.map_or_else(Task::none, open_source),
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
                    .show(Alert::error(lang::t_with("common.open_failed", &[("error", error.as_str())])))
                    .discard()
            }
        }
    }
}

/// What a save writes: an instance as it now stands, the folder its paths are relative to, and the picture the edit
/// replaced. The picture is removed once the save lands.
#[derive(Debug, Clone)]
pub(crate) struct InstanceSave {
    pub instance: Instance,
    pub folder: PathBuf,
    pub replaced: Option<PathBuf>,
}

impl LodestoneLauncher {
    /// Applies an edit the Settings tab made to the shown instance, and saves the result. A banner or icon is written to
    /// disk first, and the save follows once the file is there.
    pub(super) fn edit_instance(&mut self, edit: Edit) -> Task<LodestoneAppMessage> {
        let Some(mut instance) = self.instance_page.instance().cloned() else {
            return Task::none();
        };
        // A locked instance keeps the Minecraft version and loader its pack set. The Settings tab disables those pickers, and
        // this refuses the edit should one still arrive.
        if instance.locked && matches!(edit, Edit::MinecraftVersion(_) | Edit::LoaderBuild(_)) {
            return Task::none();
        }
        let folder = self.paths.instance(&instance.directory_name);
        let mut replaced = None;
        let mut tasks = Vec::new();
        match edit {
            Edit::Name(name) => {
                // A name is kept as typed, so the field shows what is saved. Only an empty name is refused.
                if name.trim().is_empty() {
                    return Task::none();
                }
                instance.name = name;
            }
            Edit::Launch(change) => instance.settings.override_with(change),
            Edit::Snapshots(on) => instance.settings.snapshots = on,
            Edit::Reset => instance.settings.reset(),
            Edit::MinecraftVersion(version) => {
                // Vanilla starts the version itself. A modded instance installs its loader again at its next launch.
                instance.version_id = (instance.loader == Loader::Vanilla).then(|| version.clone());
                instance.mc_version = version;
                instance.loader_version = None;
                tasks.push(Task::done(LodestoneAppMessage::Instance(InstanceMessage::Settings(
                    settings::Message::LoadBuilds,
                ))));
            }
            Edit::LoaderBuild(version) => {
                instance.loader_version = Some(version);
                instance.version_id = None;
            }
            Edit::Banner(BannerChoice::Scene(index)) => {
                let (biome, seed) = banner::scene(index);
                replaced = instance.art_path.take();
                instance.biome = biome;
                instance.art_seed = seed;
            }
            Edit::Banner(BannerChoice::Uploaded(upload)) => {
                return Task::perform(write_art(folder, "banner", upload.png), |result| {
                    LodestoneAppMessage::Instance(InstanceMessage::Settings(settings::Message::Edit(Edit::BannerWritten(result))))
                });
            }
            Edit::BannerWritten(Ok(relative)) => replaced = instance.art_path.replace(folder.join(relative)),
            Edit::BannerWritten(Err(error)) => return self.art_failed(&error),
            Edit::Icon(IconChoice::Composed { background, symbol }) => {
                replaced = custom_icon(&instance.icon);
                instance.icon = InstanceIcon::Composed {
                    background: background.to_string(),
                    symbol: symbol.to_string(),
                };
                tasks.push(self.record_recent_icon(background, symbol));
            }
            Edit::Icon(IconChoice::Uploaded(upload)) => {
                return Task::perform(write_art(folder, "icon", upload.png), |result| {
                    LodestoneAppMessage::Instance(InstanceMessage::Settings(settings::Message::Edit(Edit::IconWritten(result))))
                });
            }
            Edit::IconWritten(Ok(relative)) => {
                replaced = custom_icon(&instance.icon);
                instance.icon = InstanceIcon::Custom(folder.join(relative));
            }
            Edit::IconWritten(Err(error)) | Edit::Failed(error) => return self.art_failed(&error),
        }
        tasks.push(self.save_instance(instance, replaced));
        Task::batch(tasks)
    }

    /// Saves an instance: the library copy and the page take it at once, and the database write runs behind them.
    /// Writes for one instance are kept in order (see [`KeyedWriter`]).
    pub(super) fn save_instance(&mut self, instance: Instance, replaced: Option<PathBuf>) -> Task<LodestoneAppMessage> {
        let folder = self.paths.instance(&instance.directory_name);
        let id = instance.id.clone();
        self.library.add_instance(instance.clone());
        self.refresh_instance_page();
        let save = InstanceSave { instance, folder, replaced };
        match self.instance_saves.request(id.clone(), save) {
            Some(save) => self.write_instance(id, save),
            None => Task::none(),
        }
    }

    /// Writes one save to the database, then removes the picture it replaced.
    fn write_instance(&self, id: String, save: InstanceSave) -> Task<LodestoneAppMessage> {
        let Some(db) = self.db.clone() else {
            return Task::none();
        };
        Task::perform(
            async move {
                let result = db.save_instance(&save.instance, &save.folder).await;
                if result.is_ok()
                    && let Some(old) = save.replaced
                {
                    let removed = tokio::task::spawn_blocking(move || std::fs::remove_file(&old)).await;
                    if let Ok(Err(error)) = removed
                        && error.kind() != ErrorKind::NotFound
                    {
                        warn!("Removing the replaced picture failed: {error}");
                    }
                }
                result
            },
            move |result| LodestoneAppMessage::InstanceSaved {
                instance: id.clone(),
                result,
            },
        )
    }

    /// Finishes a save. The newest save that arrived during it is written next. A failed save is reported, and the
    /// library reloads from the database, so the page shows what was stored.
    pub(super) fn instance_saved(&mut self, id: String, result: Result<(), String>) -> Task<LodestoneAppMessage> {
        let mut tasks = Vec::new();
        if let Some(next) = self.instance_saves.complete(&id) {
            tasks.push(self.write_instance(id.clone(), next));
        }
        if let Err(error) = result {
            warn!("Saving instance {id} failed: {error}");
            tasks.push(
                self.alerts
                    .show(Alert::error(lang::t_with("instance.settings.save_failed", &[("error", error.as_str())])))
                    .discard(),
            );
            tasks.push(self.load_instances());
        }
        Task::batch(tasks)
    }

    /// Asks before every launch setting of the shown instance goes back to the global defaults. The answer returns as
    /// `InstanceResetAnswered`.
    pub(super) fn ask_instance_reset(&mut self) -> Task<LodestoneAppMessage> {
        let Some(instance) = self.instance_page.instance() else {
            return Task::none();
        };
        let id = instance.id.clone();
        let message = lang::t_with("instance.settings.reset_confirm", &[("name", instance.name.as_str())]);
        self.alerts
            .show(Alert::danger(message).title(lang::t("instance.settings.reset_title")))
            .map(move |confirmed| LodestoneAppMessage::InstanceResetAnswered {
                instance: id.clone(),
                confirmed,
            })
    }

    /// Resets the launch settings of the instance `id` when the player confirmed it.
    pub(super) fn instance_reset_answered(&mut self, id: String, confirmed: bool) -> Task<LodestoneAppMessage> {
        let shown = self.instance_page.instance().is_some_and(|instance| instance.id == id);
        if confirmed && shown {
            return self.edit_instance(Edit::Reset);
        }
        Task::none()
    }

    /// Reports a banner or icon that could not be written.
    fn art_failed(&mut self, error: &str) -> Task<LodestoneAppMessage> {
        warn!("Writing the picture failed: {error}");
        self.alerts
            .show(Alert::error(lang::t_with("instance.settings.art_failed", &[("error", error)])))
            .discard()
    }

    /// Records a composed icon as the most recent one, for the icon editor's recents.
    fn record_recent_icon(&self, background: &str, symbol: &str) -> Task<LodestoneAppMessage> {
        let Some(db) = self.db.clone() else {
            return Task::none();
        };
        let (background, symbol) = (background.to_string(), symbol.to_string());
        Task::future(async move {
            if let Err(error) = db.record_recent_icon(&background, &symbol).await {
                warn!("could not record the icon as recent: {error}");
            }
        })
        .discard()
    }
}

/// The uploaded icon the instance keeps, if it has one. A save replaces it.
fn custom_icon(icon: &InstanceIcon) -> Option<PathBuf> {
    match icon {
        InstanceIcon::Custom(path) => Some(path.clone()),
        InstanceIcon::Composed { .. } => None,
    }
}

/// Writes a picture into the instance folder's art folder, under a name that has not been used. A new name keeps the
/// image cache from showing the picture the old name held. Returns the path relative to the instance folder.
async fn write_art(folder: PathBuf, kind: &'static str, png: Vec<u8>) -> Result<PathBuf, String> {
    tokio::task::spawn_blocking(move || {
        let relative = PathBuf::from(ART_DIR).join(format!("{kind}-{}.png", Utc::now().timestamp_millis()));
        std::fs::create_dir_all(folder.join(ART_DIR)).map_err(|error| error.to_string())?;
        lodestone_fs::write_atomic(&folder.join(&relative), &png).map_err(|error| error.to_string())?;
        Ok(relative)
    })
    .await
    .map_err(|error| error.to_string())?
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

/// Opens `url` in the browser on the blocking pool. A failure returns as `SourceOpened`. Only a web address is opened: a link a
/// project, a news post or a release names is written by whoever published it, and a file or a registered program must not be
/// started from it (see [`is_web_link`]).
pub(super) fn open_source(url: String) -> Task<LodestoneAppMessage> {
    let url = url.trim().to_string();
    if !is_web_link(&url) {
        tracing::warn!("refusing to open a link that is not an http or https address");
        return Task::none();
    }
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

/// Removes the instance's folder, then the folders the launcher keeps for it outside the instance, then its row. The folders
/// go first: if a file is locked, the row stays and the delete can be tried again.
async fn remove_instance(db: Database, folder: PathBuf, kept: Vec<PathBuf>, id: String) -> Result<(), String> {
    let removal = tokio::task::spawn_blocking(move || {
        remove_folder(&folder)?;
        kept.iter().try_for_each(|path| remove_folder(path))
    })
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

/// Whether `text` is an `http` or `https` address. The only links the launcher opens.
pub fn is_web_link(text: &str) -> bool {
    reqwest::Url::parse(text).is_ok_and(|url| matches!(url.scheme(), "http" | "https"))
}
