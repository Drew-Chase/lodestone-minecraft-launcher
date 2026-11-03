//! The Import Instance dialog. It reads a modpack file, a CurseForge page, a Lodestone backup, or an instance another launcher
//! keeps, and queues the install that makes its instance. The dialog keeps the imports it queued, so the latest can be started
//! again.
//!
//! This module is the root's side: opening and closing the dialog, starting a read, and queueing what the read answers with.
//! The dialog is in `dialog/`, the reads are in `prepare.rs`, the CurseForge address is read by `page.rs`, and the other
//! launchers' folders are read by `launcher/`.

mod dialog;
mod launcher;
mod page;
mod prepare;
pub mod record;

use std::path::PathBuf;

use chrono::{Duration, Local};
use iced::{Event, Task, event, window};
use tracing::warn;

use super::{LodestoneAppMessage, LodestoneLauncher};
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
    /// The From launcher tab opened: the launchers on this machine are looked for, in the background.
    ScanLaunchers,
    /// The launchers were looked for, with what each folder held.
    Scanned(Vec<launcher::Scan>),
    /// The Locate button of a launcher was pressed: the folder picker opens.
    Locate(launcher::Launcher),
    /// The folder picker answered for a launcher, or was cancelled.
    LocatePicked(launcher::Launcher, Option<PathBuf>),
    /// The folder a launcher was located in was checked, and listed when it holds the launcher's instances.
    Located(launcher::Launcher, Result<launcher::Scan, String>),
    /// An instance's checkbox was pressed.
    ToggleInstance(launcher::Pick),
    /// A launcher's select-all checkbox was pressed.
    ToggleLauncher(launcher::Launcher),
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
            Err(error) => {
                if let Some(dialog) = self.import.as_mut() {
                    dialog.failed(error);
                }
                Task::none()
            }
        }
    }

    /// The folder names a new instance must avoid: the ones queued installs reserve, and the ones the library holds.
    fn taken_directories(&self) -> Vec<String> {
        self.downloads
            .reserved_directories()
            .into_iter()
            .chain(self.library.directory_names())
            .collect()
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
