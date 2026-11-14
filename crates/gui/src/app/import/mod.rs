//! The Import Instance dialog. It reads a modpack file, a CurseForge page, a Lodestone backup, or an instance another launcher
//! keeps, and queues the install that makes its instance. The dialog keeps the imports it queued, so the latest can be started
//! again.
//!
//! This module is the root's side: opening and closing the dialog, starting a read, and queueing what the read answers with.
//! The dialog is in `dialog/`, the reads are in `prepare.rs`, the CurseForge address is read by `page.rs`, and the other
//! launchers' folders are read by `launcher/`.

mod dialog;
mod launcher;
pub(crate) mod launcher_picker;
mod page;
mod prepare;
pub mod record;

use std::path::PathBuf;

use chrono::{Duration, Local};
use iced::{Event, Task, event, window};
use tracing::warn;

use super::{LodestoneAppMessage, LodestoneLauncher};
use crate::app::components::alert::Alert;
use crate::app::pages::setup::Message as SetupMessage;
use dialog::Tab;
use prepare::{Picked, Prepared};
use record::ImportRecord;

pub use dialog::Dialog;

/// How many recent imports the dialog lists, and how far back it looks for them, in days.
const RECENT_LIMIT: i64 = 3;
const RECENT_DAYS: i64 = 7;

/// The dialog's messages: its tabs and fields, the files it is given, the reads it starts, and their answers.
#[derive(Debug, Clone)]
pub enum Message {
    /// Closes the dialog. A read still under way is abandoned.
    Close,
    /// A tab was picked.
    TabSelected(Tab),
    /// The address field changed.
    UrlChanged(String),
    /// A file is over the window (`true`), or has left it (`false`).
    Hovering(bool),
    /// A file was dropped on the window.
    Dropped(PathBuf),
    /// The zone or its link was pressed: the file picker opens.
    Browse,
    /// The file picker answered with a path, or `None` when it was cancelled.
    Picked(Option<PathBuf>),
    /// The file the dialog was given was checked, or refused.
    Inspected(Result<Picked, String>),
    /// Import was pressed, or Enter in the address field: the import is read and queued.
    Submit,
    /// A recent import was picked again.
    Reimport(usize),
    /// The recent imports were read from the database.
    Recent(Result<Vec<ImportRecord>, String>),
    /// The import was read into the installs that queue it, or could not be.
    Prepared(Result<Vec<Prepared>, String>),
    /// The From launcher tab's picker: the launchers are looked for and located, and their instances checked.
    Picker(launcher_picker::Message),
}

impl LodestoneLauncher {
    /// Opens the Import dialog and reads the recent imports it lists.
    pub(super) fn open_import(&mut self) -> Task<LodestoneAppMessage> {
        self.import = Some(Dialog::default());
        let Some(db) = self.db.clone() else {
            return Task::none();
        };
        let since = Local::now() - Duration::days(RECENT_DAYS);
        Task::perform(async move { db.recent_imports(since, RECENT_LIMIT).await }, |result| {
            LodestoneAppMessage::Import(Message::Recent(result))
        })
    }

    /// Applies a dialog message. Closing, starting a read and its answer belong to the root, since they touch the queue,
    /// the database and the library. The rest go to the dialog.
    pub(super) fn import_message(&mut self, message: Message) -> Task<LodestoneAppMessage> {
        match message {
            Message::Close => {
                if let Some(mut dialog) = self.import.take() {
                    dialog.abandon();
                }
                Task::none()
            }
            Message::Submit => self.start_import(None),
            Message::Reimport(index) => self.start_import(Some(index)),
            Message::Prepared(result) => self.import_prepared(result),
            message => match self.import.as_mut() {
                Some(dialog) => dialog.update(message).map(LodestoneAppMessage::Import),
                None => Task::none(),
            },
        }
    }

    /// Starts reading the import the dialog holds, or the recent one at `reimport`. A read already under way is not started
    /// again, and a tab with nothing in it starts nothing.
    fn start_import(&mut self, reimport: Option<usize>) -> Task<LodestoneAppMessage> {
        let Some(dialog) = self.import.as_ref().filter(|dialog| !dialog.busy()) else {
            return Task::none();
        };
        let Some(work) = dialog.work(reimport) else {
            return Task::none();
        };
        let read = Task::perform(prepare::read(work, self.paths.clone(), self.taken_directories()), |result| {
            LodestoneAppMessage::Import(Message::Prepared(result))
        });
        let (task, abort) = read.abortable();
        if let Some(dialog) = self.import.as_mut() {
            dialog.begin(abort);
        }
        task
    }

    /// Queues the installs a read answered with, records each for the recent list, and closes the dialog. A read that failed
    /// stays in the dialog, with its reason.
    fn import_prepared(&mut self, result: Result<Vec<Prepared>, String>) -> Task<LodestoneAppMessage> {
        match result {
            Ok(prepared) => {
                self.import = None;
                self.queue_prepared(prepared)
            }
            Err(error) => {
                if let Some(dialog) = self.import.as_mut() {
                    dialog.failed(error);
                }
                Task::none()
            }
        }
    }

    /// Queues the installs a read answered with, and records each import for the recent list. The setup wizard's Import step
    /// queues its reads the same way.
    pub(super) fn queue_prepared(&mut self, prepared: Vec<Prepared>) -> Task<LodestoneAppMessage> {
        let mut tasks = Vec::with_capacity(prepared.len() * 2);
        for Prepared { request, record } in prepared {
            tasks.push(self.queue_install(request));
            if let Some(db) = self.db.clone() {
                tasks.push(
                    Task::future(async move {
                        if let Err(error) = db.record_import(&record).await {
                            warn!("recording the import failed: {error}");
                        }
                    })
                    .discard(),
                );
            }
        }
        Task::batch(tasks)
    }

    /// The folder names a new instance must avoid: the ones queued installs reserve, and the ones the library holds.
    pub(super) fn taken_directories(&self) -> Vec<String> {
        self.downloads
            .reserved_directories()
            .into_iter()
            .chain(self.library.directory_names())
            .collect()
    }
}

impl LodestoneLauncher {
    /// Applies a message of the setup wizard's Import step: the picker's, and the read and queueing of the checked instances.
    pub(super) fn setup_import_message(&mut self, message: Message) -> Task<LodestoneAppMessage> {
        match message {
            Message::Picker(message) => self
                .setup
                .launchers_mut()
                .update(message)
                .map(|message| LodestoneAppMessage::Setup(SetupMessage::Import(Message::Picker(message)))),
            Message::Submit => self.setup_import_submit(),
            Message::Prepared(result) => self.setup_import_prepared(result),
            _ => Task::none(),
        }
    }

    /// Looks for the launchers when the wizard's Import step is first shown. A look that is already under way is not repeated.
    pub(super) fn setup_import_scan(&mut self) -> Task<LodestoneAppMessage> {
        let picker = self.setup.launchers();
        if picker.scans.is_empty() && !picker.scanning {
            return self.setup_import_message(Message::Picker(launcher_picker::Message::Scan));
        }
        Task::none()
    }

    /// Reads the instances the wizard's picker has checked, on the blocking pool. The read answers with `Prepared`.
    fn setup_import_submit(&mut self) -> Task<LodestoneAppMessage> {
        if self.setup.importing() {
            return Task::none();
        }
        let Some(work) = self.setup.launchers().work() else {
            return Task::none();
        };
        self.setup.set_importing(true);
        let read = prepare::read(work, self.paths.clone(), self.taken_directories());
        Task::perform(read, |result| LodestoneAppMessage::Setup(SetupMessage::Import(Message::Prepared(result))))
    }

    /// The read answered. Its installs are queued and the checked instances are cleared, or the reason it failed is shown.
    fn setup_import_prepared(&mut self, result: Result<Vec<Prepared>, String>) -> Task<LodestoneAppMessage> {
        self.setup.set_importing(false);
        match result {
            Ok(prepared) => {
                self.setup.add_imported(prepared.len());
                self.setup.launchers_mut().clear_chosen();
                self.queue_prepared(prepared)
            }
            Err(error) => self.alerts.show(Alert::error(error)).discard(),
        }
    }
}

/// The window's file events, which the dialog listens for while it is open: a file over the window, a file that leaves
/// it, and a file dropped on it.
pub(crate) fn drop_event(event: Event, _status: event::Status, _window: window::Id) -> Option<LodestoneAppMessage> {
    let message = match event {
        Event::Window(window::Event::FileHovered(_)) => Message::Hovering(true),
        Event::Window(window::Event::FilesHoveredLeft) => Message::Hovering(false),
        Event::Window(window::Event::FileDropped(path)) => Message::Dropped(path),
        _ => return None,
    };
    Some(LodestoneAppMessage::Import(message))
}
