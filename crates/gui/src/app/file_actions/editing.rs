//! The editor's side of the Files tab: reading a file into it, writing it back, and the questions asked before
//! unsaved changes are lost.
//!
//! A file is read on the blocking pool and refused when it is not text or too large to edit, in which case the system's
//! own app is offered. A save checks that the file on disk is still the one that was read, so a config the game
//! rewrote meanwhile is never silently overwritten. Leaving the editor with unsaved changes is always asked about.

use std::path::{Path, PathBuf};

use iced::Task;
use tracing::{info, warn};

use super::{Message, name_of};
use crate::app::components::alert::Alert;
use crate::app::components::editor::encoding::{self, Encoding};
use crate::app::components::editor::{Loaded, MAX_BYTES, Stamp};
use crate::app::lang;
use crate::app::pages::instance::Tab;
use crate::app::pages::instance::browser::ops;
use crate::app::reveal::open_file;
use crate::app::{LodestoneAppMessage, LodestoneLauncher};

/// Why a file is being read into the editor.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Purpose {
    /// To open it.
    Open,
    /// To replace the text on screen with the file as it is now.
    Reload,
}

/// Why a file could not be read into the editor.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ReadError {
    /// The file is larger than the editor opens.
    TooLarge,
    /// The file's bytes are not text.
    NotText,
    /// The file system refused, with its own words.
    Io(String),
}

/// Why the editor's text could not be written.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum SaveError {
    /// The file on disk is not the one the editor read, so writing would lose what is there.
    Changed,
    /// The file system refused, with its own words.
    Io(String),
}

/// What was about to happen when unsaved changes stood in the way. It goes ahead once the user gives the changes up.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Leaving {
    /// Another file was opened, by its path inside the instance.
    OpenFile(String),
    /// The file was to be read again.
    Revert,
    /// The editor was closed.
    CloseEditor,
    /// An instance page was opened on a tab.
    OpenInstance { id: String, tab: Tab },
    /// The launcher's window was closed.
    CloseWindow,
}

/// When the file at `path` was last written and how long it is. `None` when it cannot be read, as a file that is gone
/// cannot.
pub fn stamp_of(path: &Path) -> Option<Stamp> {
    let metadata = std::fs::metadata(path).ok()?;
    Some(Stamp {
        modified: metadata.modified().ok(),
        length: metadata.len(),
    })
}

/// Reads the file at `absolute` as text. `path` is its path inside the instance, which the editor shows.
pub fn read_text(absolute: &Path, path: String) -> Result<Loaded, ReadError> {
    let metadata = std::fs::metadata(absolute).map_err(|error| ReadError::Io(error.to_string()))?;
    if metadata.len() > MAX_BYTES {
        return Err(ReadError::TooLarge);
    }
    // The stamp is taken before the read, so a write that lands during the read shows as a change afterwards.
    let stamp = Stamp {
        modified: metadata.modified().ok(),
        length: metadata.len(),
    };
    let bytes = std::fs::read(absolute).map_err(|error| ReadError::Io(error.to_string()))?;
    let (text, encoding) = encoding::decode(&bytes).ok_or(ReadError::NotText)?;
    Ok(Loaded { path, text, encoding, stamp })
}

/// Writes `bytes` to the file at `absolute`, unless the file is no longer the one stamped `known`. `force` writes
/// whatever the file holds now. A file that is gone is written again. Returns the stamp of the file as written.
pub fn write_text(absolute: &Path, bytes: &[u8], known: Stamp, force: bool) -> Result<Stamp, SaveError> {
    if !force && stamp_of(absolute).is_some_and(|current| current != known) {
        return Err(SaveError::Changed);
    }
    lodestone_fs::write_atomic(absolute, bytes).map_err(|error| SaveError::Io(error.to_string()))?;
    stamp_of(absolute).ok_or_else(|| SaveError::Io(String::from("the file could not be read back")))
}

impl LodestoneLauncher {
    /// The absolute path of `path` inside the instance the Files tab shows.
    fn file_inside(&self, path: &str) -> Option<PathBuf> {
        ops::inside(self.instance_page.browser().root(), path)
    }

    /// The question to ask before `leaving` goes ahead, when the editor holds unsaved changes. `None` when it holds
    /// none, and `leaving` can go ahead at once.
    pub(in crate::app) fn unsaved_guard(&mut self, leaving: Leaving) -> Option<Task<LodestoneAppMessage>> {
        let editor = self.instance_page.browser().editor().filter(|editor| editor.is_dirty())?;
        let name = name_of(Path::new(editor.path()));
        let message = lang::t_with("editor.unsaved_question", &[("name", name.as_str())]);
        Some(
            self.alerts
                .show(
                    Alert::warn(message)
                        .title(lang::t("editor.unsaved_title"))
                        .labelled(lang::t("editor.discard"), Some(lang::t("editor.keep_editing"))),
                )
                .map(move |confirmed| {
                    LodestoneAppMessage::Files(Message::LeaveAnswered {
                        leaving: leaving.clone(),
                        confirmed,
                    })
                }),
        )
    }

    /// The answer to the unsaved-changes question. A yes gives the changes up, and what was asked for goes ahead.
    pub(super) fn leave_answered(&mut self, leaving: Leaving, confirmed: bool) -> Task<LodestoneAppMessage> {
        if !confirmed {
            return Task::none();
        }
        match leaving {
            Leaving::OpenFile(path) => self.read_into_editor(path, Purpose::Open),
            Leaving::Revert => match self.instance_page.browser().editor().map(|editor| editor.path().to_string()) {
                Some(path) => self.read_into_editor(path, Purpose::Reload),
                None => Task::none(),
            },
            Leaving::CloseEditor => {
                self.instance_page.browser_mut().close_editor();
                Task::none()
            }
            Leaving::OpenInstance { id, tab } => {
                self.instance_page.browser_mut().close_editor();
                self.open_instance_page(&id, tab)
            }
            Leaving::CloseWindow => {
                self.instance_page.browser_mut().close_editor();
                self.close_requested()
            }
        }
    }

    /// Opens the file at `path` in the editor. The file already open there stays, and unsaved changes to another file are
    /// asked about first.
    pub(super) fn open_in_editor(&mut self, path: String) -> Task<LodestoneAppMessage> {
        if self.instance_page.browser().editor().is_some_and(|editor| editor.path() == path) {
            return Task::none();
        }
        match self.unsaved_guard(Leaving::OpenFile(path.clone())) {
            Some(ask) => ask,
            None => self.read_into_editor(path, Purpose::Open),
        }
    }

    /// Reads the file at `path` on the blocking pool. The text lands as `Read`.
    fn read_into_editor(&mut self, path: String, purpose: Purpose) -> Task<LodestoneAppMessage> {
        let Some(absolute) = self.file_inside(&path) else {
            return Task::none();
        };
        let generation = self.instance_page.generation();
        let answered = path.clone();
        Task::perform(
            async move {
                tokio::task::spawn_blocking(move || read_text(&absolute, path))
                    .await
                    .unwrap_or_else(|error| Err(ReadError::Io(error.to_string())))
            },
            move |result| {
                LodestoneAppMessage::Files(Message::Read {
                    generation,
                    purpose,
                    path: answered.clone(),
                    result,
                })
            },
        )
    }

    /// Takes a file that was read: it opens in the editor, or replaces the text there. A file the editor cannot hold is
    /// offered to the system's own app instead.
    pub(super) fn file_read(&mut self, purpose: Purpose, path: String, result: Result<Loaded, ReadError>) -> Task<LodestoneAppMessage> {
        let browser = self.instance_page.browser_mut();
        match (result, purpose) {
            (Ok(loaded), Purpose::Open) => {
                browser.open_editor(loaded);
                Task::none()
            }
            (Ok(loaded), Purpose::Reload) => {
                if let Some(editor) = browser.editor_mut().filter(|editor| editor.path() == path) {
                    editor.reload(loaded);
                }
                Task::none()
            }
            (Err(ReadError::Io(error)), _) => {
                let text = lang::t_with(
                    "editor.read_failed",
                    &[("name", name_of(Path::new(&path)).as_str()), ("error", error.as_str())],
                );
                self.alerts.show(Alert::error(text)).discard()
            }
            (Err(reason), _) => {
                let key = if reason == ReadError::TooLarge {
                    "editor.too_large"
                } else {
                    "editor.not_text"
                };
                let message = lang::t_with(key, &[("name", name_of(Path::new(&path)).as_str())]);
                let generation = self.instance_page.generation();
                self.alerts
                    .show(Alert::info(message).labelled(lang::t("instance.files.menu.open_external"), Some(lang::t("common.cancel"))))
                    .map(move |confirmed| {
                        LodestoneAppMessage::Files(Message::ExternalAnswered {
                            generation,
                            path: path.clone(),
                            confirmed,
                        })
                    })
            }
        }
    }

    /// Opens the file in the system's own app, once the user has said so.
    pub(super) fn external_answered(&mut self, path: String, confirmed: bool) -> Task<LodestoneAppMessage> {
        match self.file_inside(&path).filter(|_| confirmed) {
            Some(file) => open_file(file),
            None => Task::none(),
        }
    }

    /// Writes the editor's text to its file on the blocking pool. Without `force` the write is refused when the file on
    /// disk is not the one the editor read. The outcome lands as `Saved`.
    pub(super) fn save_editor(&mut self, force: bool) -> Task<LodestoneAppMessage> {
        let Some(editor) = self.instance_page.browser().editor().filter(|editor| !editor.read_only()) else {
            return Task::none();
        };
        let path = editor.path().to_string();
        let Some(absolute) = self.file_inside(&path) else {
            return Task::none();
        };
        let (bytes, encoding) = editor.bytes();
        let known = editor.stamp();
        let generation = self.instance_page.generation();
        Task::perform(
            async move {
                tokio::task::spawn_blocking(move || write_text(&absolute, &bytes, known, force).map(|stamp| (encoding, stamp)))
                    .await
                    .unwrap_or_else(|error| Err(SaveError::Io(error.to_string())))
            },
            move |result| {
                LodestoneAppMessage::Files(Message::Saved {
                    generation,
                    path: path.clone(),
                    result,
                })
            },
        )
    }

    /// Applies a save's outcome. A file that changed on disk is asked about before it is overwritten.
    pub(super) fn file_saved(&mut self, path: String, result: Result<(Encoding, Stamp), SaveError>) -> Task<LodestoneAppMessage> {
        match result {
            Ok((encoding, stamp)) => {
                info!("Saved {path}");
                if let Some(editor) = self.instance_page.browser_mut().editor_mut().filter(|editor| editor.path() == path) {
                    editor.saved(encoding, stamp);
                }
                self.refresh_files()
            }
            Err(SaveError::Changed) => {
                let generation = self.instance_page.generation();
                let message = lang::t_with("editor.overwrite_question", &[("name", name_of(Path::new(&path)).as_str())]);
                self.alerts
                    .show(
                        Alert::warn(message)
                            .title(lang::t("editor.overwrite_title"))
                            .labelled(lang::t("editor.overwrite"), Some(lang::t("common.cancel"))),
                    )
                    .map(move |confirmed| LodestoneAppMessage::Files(Message::OverwriteAnswered { generation, confirmed }))
            }
            Err(SaveError::Io(error)) => {
                warn!("Saving {path} failed: {error}");
                let text = lang::t_with(
                    "editor.save_failed",
                    &[("name", name_of(Path::new(&path)).as_str()), ("error", error.as_str())],
                );
                self.alerts.show(Alert::error(text)).discard()
            }
        }
    }

    /// Closes the editor, after asking about unsaved changes.
    pub(super) fn close_editor(&mut self) -> Task<LodestoneAppMessage> {
        match self.unsaved_guard(Leaving::CloseEditor) {
            Some(ask) => ask,
            None => {
                self.instance_page.browser_mut().close_editor();
                Task::none()
            }
        }
    }

    /// Reads the editor's file again in place of the text on screen, after asking about unsaved changes.
    pub(super) fn revert_editor(&mut self) -> Task<LodestoneAppMessage> {
        let Some(path) = self.instance_page.browser().editor().map(|editor| editor.path().to_string()) else {
            return Task::none();
        };
        match self.unsaved_guard(Leaving::Revert) {
            Some(ask) => ask,
            None => self.read_into_editor(path, Purpose::Reload),
        }
    }

    /// Looks at the editor's file on disk, on the blocking pool. What is found lands as `Stat`.
    pub(super) fn check_open_file(&mut self) -> Task<LodestoneAppMessage> {
        let Some(path) = self.instance_page.browser().editor().map(|editor| editor.path().to_string()) else {
            return Task::none();
        };
        let Some(absolute) = self.file_inside(&path) else {
            return Task::none();
        };
        let generation = self.instance_page.generation();
        Task::perform(
            async move { tokio::task::spawn_blocking(move || stamp_of(&absolute)).await.ok().flatten() },
            move |stamp| {
                LodestoneAppMessage::Files(Message::Stat {
                    generation,
                    path: path.clone(),
                    stamp,
                })
            },
        )
    }

    /// Acts on what the editor's file looks like on disk. A file that changed is read again when the text here has no
    /// changes of its own, and flagged when it has. A file that is gone closes an editor with nothing to lose.
    pub(super) fn file_stat(&mut self, path: String, stamp: Option<Stamp>) -> Task<LodestoneAppMessage> {
        let browser = self.instance_page.browser_mut();
        let Some(editor) = browser.editor_mut().filter(|editor| editor.path() == path) else {
            return Task::none();
        };
        if stamp == Some(editor.stamp()) {
            return Task::none();
        }
        if editor.is_dirty() {
            editor.mark_changed_on_disk();
            Task::none()
        } else if stamp.is_some() {
            self.read_into_editor(path, Purpose::Reload)
        } else {
            browser.close_editor();
            Task::none()
        }
    }
}

#[cfg(test)]
#[path = "../../../tests/app/file_actions/editing.rs"]
mod tests;
