//! The server pack wizard's root side: it opens the wizard from an instance's menu, starts the scan and the export,
//! keeps the mod choices the database remembers, and carries out the page's messages. The page itself is in
//! `pages/server_pack`, and it only shows what this module feeds it.

mod job;
mod scan;

use std::path::{Path, PathBuf};
use std::sync::Arc;
use std::sync::atomic::{AtomicBool, Ordering};

use iced::Task;
use lodestone_export::server_pack::plan::Choice;
use lodestone_export::server_pack::write::Output;
use lodestone_loaders::server::ServerKind;
use tracing::{info, warn};

use super::{LodestoneAppMessage, LodestoneLauncher};
use crate::app::components::alert::Alert;
use crate::app::instances::reveal_folder;
use crate::app::lang;
use crate::app::pages::Page;
use crate::app::pages::library::instance::{Instance, Loader};
use crate::app::pages::server_pack::{Kind, Message, ScanDone, ServerSetup, State, Step, stored};

impl LodestoneLauncher {
    /// Opens the server pack wizard for the instance `id`. It reads the instance's mods and folders, and the choices the
    /// database keeps, and shows the Review step once the mods are read. An instance that is running, or still
    /// installing, is refused with an alert.
    pub(super) fn open_server_pack(&mut self, id: &str) -> Task<LodestoneAppMessage> {
        let Some((name, directory, setup)) = self
            .library
            .instance(id)
            .map(|instance| (instance.name.clone(), instance.directory_name.clone(), server_setup(&instance)))
        else {
            return Task::none();
        };
        if self.games.is_active(id) {
            let message = lang::t_with("server_pack.running", &[("name", name.as_str())]);
            return self.alerts.show(Alert::warn(message)).discard();
        }
        if self.library.pending_install(id).is_some() {
            let message = lang::t_with("server_pack.installing", &[("name", name.as_str())]);
            return self.alerts.show(Alert::warn(message)).discard();
        }
        // A wizard that is still scanning for another opening stops its scan.
        self.server_pack.cancel.store(true, Ordering::SeqCst);
        let folder = self.paths.instance(&directory);
        let cancel = Arc::new(AtomicBool::new(false));
        self.server_pack = State::new(id.to_string(), name, folder.clone(), cancel.clone());
        self.server_pack.setup = Some(setup);
        info!("Opening the server pack wizard for {}", folder.display());
        let open = self.navigate(Page::ServerPack);
        let scanned_folder = folder.clone();
        let scan = iced::Task::sip(
            scan::run(folder.clone(), cancel),
            |tick| LodestoneAppMessage::ServerPack(Message::ScanTick(tick)),
            move |mods| {
                LodestoneAppMessage::ServerPack(Message::Scanned(ScanDone {
                    folder: scanned_folder.clone(),
                    mods,
                }))
            },
        );
        let listed = folder.clone();
        let folders = Task::perform(scan::list_folders(folder), move |result| {
            LodestoneAppMessage::ServerPack(Message::FoldersListed(listed.clone(), result))
        });
        let choices = match self.db.clone() {
            Some(db) => Task::perform(async move { db.server_pack_choices().await }, |result| {
                LodestoneAppMessage::ServerPack(Message::ChoicesLoaded(result))
            }),
            None => Task::none(),
        };
        Task::batch([open, scan, folders, choices])
    }

    /// Carries out one message of the server pack wizard. The page takes the ones that only change its own state.
    pub(super) fn server_pack_message(&mut self, message: Message) -> Task<LodestoneAppMessage> {
        match message {
            Message::Back => {
                if self.server_pack.step == Step::Scanning {
                    self.server_pack.cancel.store(true, Ordering::SeqCst);
                }
                Task::done(LodestoneAppMessage::NavBack)
            }
            Message::Cancel => match self.server_pack.step {
                Step::Scanning => {
                    self.server_pack.cancel.store(true, Ordering::SeqCst);
                    Task::done(LodestoneAppMessage::NavBack)
                }
                // The export ends by itself once the write stops, and the page then shows the step it stopped at.
                Step::Exporting => {
                    self.server_pack.cancel.store(true, Ordering::SeqCst);
                    Task::none()
                }
                _ => Task::none(),
            },
            Message::Rescan => {
                let id = self.server_pack.instance.clone();
                self.open_server_pack(&id)
            }
            Message::PickDestination => {
                let start = self
                    .server_pack
                    .destination
                    .clone()
                    .or_else(|| self.server_pack.folder.parent().map(Path::to_path_buf));
                Task::perform(choose_destination(start), |picked| {
                    LodestoneAppMessage::ServerPack(Message::DestinationPicked(picked))
                })
            }
            Message::DestinationPicked(Some(folder)) => {
                self.server_pack.destination = Some(folder);
                Task::none()
            }
            Message::DestinationPicked(None) => Task::none(),
            Message::Export => self.start_server_pack_export(),
            Message::Reveal => match self.server_pack.receipt.as_ref() {
                Some(receipt) => reveal_folder(reveal_target(&receipt.target)),
                None => Task::none(),
            },
            Message::Choose { mods, choice } => self.choose_server_pack_mods(&mods, choice),
            Message::ScanTick(tick) => {
                self.server_pack.scan_tick(tick);
                Task::none()
            }
            Message::Scanned(done) => self.server_pack_scanned(done),
            Message::ChoicesLoaded(Ok(rows)) => {
                self.server_pack.remember(rows);
                Task::none()
            }
            Message::ChoicesLoaded(Err(error)) => {
                warn!("reading the server pack choices failed: {error}");
                Task::none()
            }
            Message::FoldersListed(folder, result) => {
                if folder == self.server_pack.folder {
                    match result {
                        Ok(rows) => self.server_pack.folders_listed(rows),
                        Err(error) => warn!("listing the instance folder for the server pack failed: {error}"),
                    }
                }
                Task::none()
            }
            Message::ExportTick(tick) => {
                self.server_pack.export_tick(tick);
                Task::none()
            }
            Message::Exported(result) => self.server_pack_exported(result),
            Message::ChoiceSaved(Err(error)) => {
                warn!("remembering a server pack choice failed: {error}");
                Task::none()
            }
            Message::ChoiceSaved(Ok(())) => Task::none(),
            other => {
                self.server_pack.update(other);
                Task::none()
            }
        }
    }

    /// Takes the scan's result, when it is the scan the wizard waits for. An error is shown, and the page is left, unless
    /// the scan was stopped on purpose.
    fn server_pack_scanned(&mut self, done: ScanDone) -> Task<LodestoneAppMessage> {
        if !self.server_pack.scanning_for(&done.folder) {
            return Task::none();
        }
        match done.mods {
            Ok(mods) => {
                self.server_pack.scanned(mods);
                Task::none()
            }
            Err(error) => {
                if self.server_pack.cancel.load(Ordering::SeqCst) {
                    return Task::none();
                }
                warn!("reading the mods for the server pack failed: {error}");
                let alert = self
                    .alerts
                    .show(Alert::error(lang::t_with("server_pack.scan.failed", &[("error", error.as_str())])))
                    .discard();
                Task::batch([alert, Task::done(LodestoneAppMessage::NavBack)])
            }
        }
    }

    /// Starts writing the pack the Export step describes, once nothing blocks it.
    fn start_server_pack_export(&mut self) -> Task<LodestoneAppMessage> {
        if self.server_pack.export_blocker().is_some() {
            return Task::none();
        }
        let Some(target) = self.server_pack.target() else {
            return Task::none();
        };
        let output = match self.server_pack.kind {
            Kind::Folder => Output::Folder(target),
            Kind::Zip => Output::Zip(target),
        };
        let cancel = Arc::new(AtomicBool::new(false));
        self.server_pack.cancel = cancel.clone();
        // The server is installed only when the option is ticked and the instance's loader has one the launcher installs.
        let server = match (self.server_pack.install_server, self.server_pack.setup.as_ref()) {
            (true, Some(setup)) if setup.kind != ServerKind::Quilt => Some(job::ServerRequest {
                kind: setup.kind,
                minecraft: setup.minecraft.clone(),
                loader_version: setup.loader_version.clone(),
                eula: self.server_pack.accept_eula,
                include_java: self.server_pack.include_java,
                paths: self.paths.clone(),
            }),
            _ => None,
        };
        let installing = server.is_some();
        let request = job::Request {
            folder: self.server_pack.folder.clone(),
            selection: self.server_pack.selection.clone(),
            jars: self.server_pack.planned_jars(),
            output,
            server,
        };
        self.server_pack.begin_export(installing);
        Task::sip(
            job::run(request, cancel),
            |tick| LodestoneAppMessage::ServerPack(Message::ExportTick(tick)),
            |result| LodestoneAppMessage::ServerPack(Message::Exported(result)),
        )
    }

    /// Takes the export's result. A pack that was written shows the receipt. A failure goes back to the Export step with
    /// an alert, unless the export was cancelled.
    fn server_pack_exported(&mut self, result: Result<(PathBuf, usize), String>) -> Task<LodestoneAppMessage> {
        if self.server_pack.step != Step::Exporting {
            return Task::none();
        }
        match result {
            Ok((target, files)) => {
                info!("Wrote the server pack to {}", target.display());
                self.server_pack.finish(target, files);
                Task::none()
            }
            Err(error) => {
                self.server_pack.export_failed();
                if self.server_pack.cancel.load(Ordering::SeqCst) {
                    return Task::none();
                }
                warn!("writing the server pack failed: {error}");
                self.alerts
                    .show(Alert::error(lang::t_with("server_pack.export.failed", &[("error", error.as_str())])))
                    .discard()
            }
        }
    }

    /// Sets the choice of the mods at `mods`, and remembers each one that has a mod id in the database.
    fn choose_server_pack_mods(&mut self, mods: &[usize], choice: Option<Choice>) -> Task<LodestoneAppMessage> {
        let remember = self.server_pack.choose(mods, choice);
        let Some(db) = self.db.clone() else {
            return Task::none();
        };
        Task::batch(remember.into_iter().map(|(mod_id, choice)| {
            let db = db.clone();
            let value = choice.map(stored);
            Task::perform(async move { db.set_server_pack_choice(&mod_id, value).await }, |result| {
                LodestoneAppMessage::ServerPack(Message::ChoiceSaved(result))
            })
        }))
    }
}

/// The server an instance runs as, for the Export step's server option.
fn server_setup(instance: &Instance) -> ServerSetup {
    let kind = match instance.loader {
        Loader::Vanilla => ServerKind::Vanilla,
        Loader::Fabric => ServerKind::Fabric,
        Loader::Quilt => ServerKind::Quilt,
        Loader::Forge => ServerKind::Forge,
        Loader::NeoForge => ServerKind::NeoForge,
    };
    ServerSetup {
        kind,
        minecraft: instance.mc_version.clone(),
        loader_version: instance.loader_version.clone(),
    }
}

/// The folder the pack is written into, as the user picks it, starting at `start` when there is one. `None` when they
/// cancel.
async fn choose_destination(start: Option<PathBuf>) -> Option<PathBuf> {
    let mut picker = rfd::AsyncFileDialog::new().set_title(lang::t("server_pack.export.destination.title"));
    if let Some(start) = start {
        picker = picker.set_directory(start);
    }
    picker.pick_folder().await.map(|folder| folder.path().to_path_buf())
}

/// The folder to show for a finished pack: the pack's own folder, or the folder a zip is in.
fn reveal_target(target: &Path) -> PathBuf {
    if target.is_dir() {
        target.to_path_buf()
    } else {
        target.parent().map_or_else(|| target.to_path_buf(), Path::to_path_buf)
    }
}
