//! The Files tab's actions the root carries out: opening and revealing an entry, the clipboard, pasting, copying and
//! moving entries, and deleting, renaming and making them. The tab asks for them as [`Request`]s, and what the disk and
//! the clipboard answer comes back here as [`Message`]s. The alerts, the clipboard thread, the trash and the lock on
//! an instance belong to the root.
//!
//! Every path the tab names is checked by [`ops::inside`] before it is used, so nothing outside the instance folder is
//! changed. A request that would change the folder is refused while the instance is locked, or while an install or an
//! update is writing to it.

mod editing;
mod transfer;

use std::path::{Path, PathBuf};

use iced::Task;
use tracing::{info, warn};

use super::clipboard;
use super::components::alert::Alert;
use super::components::editor::encoding::Encoding;
use super::components::editor::{Loaded, Stamp};
use super::lang;
use super::pages::instance::Message as InstanceMessage;
use super::pages::instance::browser::ops::{self, Existing, MakeError, NameError, Report, Transfer};
use super::pages::instance::browser::{Clip, Kind, Mode, Request};
use super::pages::instance::files;
use super::reveal::{open_file, reveal_file};
use super::{LodestoneAppMessage, LodestoneLauncher};

pub use editing::{Leaving, Purpose, ReadError, SaveError};
pub use transfer::paste_plan;

/// What the disk and the clipboard answered for the Files tab. Each answer carries the `generation` of the instance it
/// was asked about, and an answer for an instance the page no longer shows is dropped.
#[derive(Debug, Clone)]
pub enum Message {
    /// The files were put on the system's clipboard, or could not be.
    Clipped(Result<(), String>),
    /// The system's clipboard was read for a paste into `into`.
    ClipboardRead {
        generation: u64,
        into: String,
        result: Result<Vec<PathBuf>, String>,
    },
    /// Entries were copied or moved. `job` is what was asked.
    Transferred { generation: u64, job: Transfer, report: Report },
    /// The question about replacing the entries a transfer found in its way was answered. `job` holds only those entries.
    ReplaceAnswered { generation: u64, job: Transfer, confirmed: bool },
    /// The question about deleting entries was answered.
    DeleteAnswered { generation: u64, paths: Vec<String>, confirmed: bool },
    /// Entries were moved to the trash. `failed` names the ones that could not be, each with its reason.
    Deleted { generation: u64, failed: Vec<(String, String)> },
    /// The entry at `from`, a path inside the instance, was renamed, or could not be.
    Renamed {
        generation: u64,
        from: String,
        result: Result<PathBuf, MakeError>,
    },
    /// A new file or folder was made, or could not be.
    Created { generation: u64, result: Result<PathBuf, MakeError> },
    /// A file was read for the editor, or could not be.
    Read {
        generation: u64,
        purpose: Purpose,
        path: String,
        result: Result<Loaded, ReadError>,
    },
    /// The editor's text was written to its file, in this encoding and with this stamp, or could not be.
    Saved {
        generation: u64,
        path: String,
        result: Result<(Encoding, Stamp), SaveError>,
    },
    /// The question about overwriting a file that changed on disk was answered.
    OverwriteAnswered { generation: u64, confirmed: bool },
    /// The question about opening a file the editor cannot hold in the system's own app was answered.
    ExternalAnswered { generation: u64, path: String, confirmed: bool },
    /// The editor's file was looked at on disk. `stamp` is `None` when it is gone.
    Stat { generation: u64, path: String, stamp: Option<Stamp> },
    /// The question about giving up unsaved changes was answered. It is not tied to the instance on screen, since what
    /// was asked for may be to leave it.
    LeaveAnswered { leaving: Leaving, confirmed: bool },
}

/// The path of `absolute` inside the instance at `root`, with `/` separators, as the tab names its rows.
fn relative(root: &Path, absolute: &Path) -> Option<String> {
    let inside = absolute.strip_prefix(root).ok()?;
    let parts: Option<Vec<&str>> = inside.components().map(|part| part.as_os_str().to_str()).collect();
    Some(parts?.join("/"))
}

/// The name of the entry at `path`, for a message.
fn name_of(path: &Path) -> String {
    path.file_name()
        .map_or_else(|| path.display().to_string(), |name| name.to_string_lossy().into_owned())
}

/// Why a name was refused, in the active language.
fn name_error(error: NameError) -> String {
    match error {
        NameError::Empty => lang::t("instance.files.name.empty").to_string(),
        NameError::Character(character) => lang::t_with("instance.files.name.character", &[("character", character.to_string().as_str())]),
        NameError::Ending => lang::t("instance.files.name.ending").to_string(),
        NameError::Reserved => lang::t("instance.files.name.reserved").to_string(),
    }
}

/// Why an entry could not be renamed or made, in the active language.
fn make_error(error: &MakeError) -> String {
    match error {
        MakeError::Name(error) => name_error(*error),
        MakeError::Taken => lang::t("instance.files.name.taken").to_string(),
        MakeError::Io(error) => error.clone(),
    }
}

impl LodestoneLauncher {
    /// Carries out what the Files tab asked for.
    pub(super) fn file_request(&mut self, request: Request) -> Task<LodestoneAppMessage> {
        let changes = matches!(
            request,
            Request::Paste { .. } | Request::Transfer { .. } | Request::Delete(_) | Request::Rename { .. } | Request::Create { .. } | Request::Save
        ) || matches!(request, Request::Clip { cut: true, .. });
        if changes && let Some(refusal) = self.files_refusal() {
            return refusal;
        }
        let root = self.instance_page.browser().root().to_path_buf();
        let generation = self.instance_page.generation();
        match request {
            Request::OpenFile(path) => self.open_in_editor(path),
            Request::Save => self.save_editor(false),
            Request::Revert => self.revert_editor(),
            Request::CloseEditor => self.close_editor(),
            Request::CheckFile => self.check_open_file(),
            Request::OpenExternal(path) => match ops::inside(&root, &path) {
                Some(file) => open_file(file),
                None => Task::none(),
            },
            Request::Reveal(path) => match ops::inside(&root, &path) {
                Some(entry) => reveal_file(entry),
                None => Task::none(),
            },
            Request::CopyPath(path) => match ops::inside(&root, &path) {
                Some(entry) => iced::clipboard::write(entry.display().to_string()),
                None => Task::none(),
            },
            Request::Clip { paths, cut } => self.clip_files(&root, &paths, cut),
            Request::Paste { into } => {
                let sender = self.clipboard.sender();
                Task::perform(clipboard::read_files(sender), move |result| {
                    LodestoneAppMessage::Files(Message::ClipboardRead {
                        generation,
                        into: into.clone(),
                        result,
                    })
                })
            }
            Request::Transfer { sources, into, mode } => match ops::inside(&root, &into) {
                Some(into) => self.run_transfer(Transfer { sources, into, mode }, Existing::Report),
                None => Task::none(),
            },
            Request::Delete(paths) => self.ask_file_delete(paths),
            Request::Rename { path, name } => {
                let Some(entry) = ops::inside(&root, &path) else {
                    return Task::none();
                };
                Task::perform(
                    async move {
                        tokio::task::spawn_blocking(move || ops::rename(&entry, &name))
                            .await
                            .unwrap_or_else(|error| Err(MakeError::Io(error.to_string())))
                    },
                    move |result| {
                        LodestoneAppMessage::Files(Message::Renamed {
                            generation,
                            from: path.clone(),
                            result,
                        })
                    },
                )
            }
            Request::Create { folder, kind } => {
                let Some(folder) = ops::inside(&root, &folder) else {
                    return Task::none();
                };
                let name = match kind {
                    Kind::Folder => lang::t("instance.files.new_folder_name"),
                    Kind::File => lang::t("instance.files.new_file_name"),
                };
                Task::perform(
                    async move {
                        tokio::task::spawn_blocking(move || ops::create(&folder, name, kind))
                            .await
                            .unwrap_or_else(|error| Err(MakeError::Io(error.to_string())))
                    },
                    move |result| LodestoneAppMessage::Files(Message::Created { generation, result }),
                )
            }
        }
    }

    /// Applies what the disk or the clipboard answered.
    pub(super) fn file_message(&mut self, message: Message) -> Task<LodestoneAppMessage> {
        let fresh = |generation: u64| generation == self.instance_page.generation();
        match message {
            Message::Clipped(result) => {
                // The launcher's own copy of the list still pastes inside the launcher, so a clipboard that refused is
                // only worth a line in the log.
                if let Err(error) = result {
                    warn!("putting files on the system clipboard failed: {error}");
                }
                Task::none()
            }
            Message::ClipboardRead { generation, into, result } if fresh(generation) => self.paste_files(into, result),
            Message::Transferred { generation, job, report } if fresh(generation) => self.transferred(job, report),
            Message::ReplaceAnswered { generation, job, confirmed } if fresh(generation) => {
                if confirmed {
                    self.run_transfer(job, Existing::Replace)
                } else {
                    Task::none()
                }
            }
            Message::DeleteAnswered {
                generation,
                paths,
                confirmed,
            } if fresh(generation) => self.answer_file_delete(paths, confirmed),
            Message::Deleted { generation, failed } if fresh(generation) => {
                // The file open in the editor may be among what went.
                let refresh = Task::batch([self.refresh_files(), self.check_open_file()]);
                if failed.is_empty() {
                    return refresh;
                }
                let mut text = lang::t_count("instance.files.delete.failed", failed.len());
                for (name, error) in &failed {
                    text.push('\n');
                    text.push_str(&lang::t_with(
                        "instance.files.failed_line",
                        &[("name", name.as_str()), ("error", error.as_str())],
                    ));
                }
                Task::batch([refresh, self.alerts.show(Alert::error(text)).discard()])
            }
            Message::Renamed { generation, from, result } if fresh(generation) => match result {
                Ok(path) => {
                    info!("Renamed an entry to {}", path.display());
                    // The editor follows its file to the new name. A folder above it that was renamed is found by the
                    // check, which closes an editor whose file is no longer where it was.
                    let root = self.instance_page.browser().root().to_path_buf();
                    let renamed = relative(&root, &path);
                    if let (Some(editor), Some(renamed)) = (self.instance_page.browser_mut().editor_mut(), renamed)
                        && editor.path() == from
                    {
                        editor.set_path(renamed);
                    }
                    self.select_written(&[path]);
                    Task::batch([self.refresh_files(), self.check_open_file()])
                }
                Err(error) => {
                    let text = lang::t_with("instance.files.rename_failed", &[("error", make_error(&error).as_str())]);
                    self.alerts.show(Alert::error(text)).discard()
                }
            },
            Message::Created { generation, result } if fresh(generation) => match result {
                Ok(path) => {
                    let root = self.instance_page.browser().root().to_path_buf();
                    if let Some(made) = relative(&root, &path) {
                        self.instance_page.browser_mut().rename_when_listed(made);
                    }
                    self.refresh_files()
                }
                Err(error) => {
                    let text = lang::t_with("instance.files.create_failed", &[("error", make_error(&error).as_str())]);
                    self.alerts.show(Alert::error(text)).discard()
                }
            },
            Message::Read {
                generation,
                purpose,
                path,
                result,
            } if fresh(generation) => self.file_read(purpose, path, result),
            Message::Saved { generation, path, result } if fresh(generation) => self.file_saved(path, result),
            Message::OverwriteAnswered { generation, confirmed } if fresh(generation) => {
                if confirmed {
                    self.save_editor(true)
                } else {
                    Task::none()
                }
            }
            Message::ExternalAnswered { generation, path, confirmed } if fresh(generation) => self.external_answered(path, confirmed),
            Message::Stat { generation, path, stamp } if fresh(generation) => self.file_stat(path, stamp),
            Message::LeaveAnswered { leaving, confirmed } => self.leave_answered(leaving, confirmed),
            // Answers for an instance the page no longer shows.
            Message::Read { .. }
            | Message::Saved { .. }
            | Message::OverwriteAnswered { .. }
            | Message::ExternalAnswered { .. }
            | Message::Stat { .. }
            | Message::ClipboardRead { .. }
            | Message::Transferred { .. }
            | Message::ReplaceAnswered { .. }
            | Message::DeleteAnswered { .. }
            | Message::Deleted { .. }
            | Message::Renamed { .. }
            | Message::Created { .. } => Task::none(),
        }
    }

    /// The alert that refuses a change to the instance folder, when one has to be refused: while an install or an update
    /// writes to the instance. A locked instance is refused without a word, since its tab offers no change.
    fn files_refusal(&mut self) -> Option<Task<LodestoneAppMessage>> {
        if self.shown_locked() {
            return Some(Task::none());
        }
        let instance = self.instance_page.instance()?;
        self.downloads
            .job_for_instance(&instance.id)
            .is_some()
            .then(|| self.alerts.show(Alert::warn(lang::t("pack_update.busy.working"))).discard())
    }

    /// Reads the folder the Files tab shows again.
    fn refresh_files(&mut self) -> Task<LodestoneAppMessage> {
        self.instance_page
            .browser_mut()
            .refresh()
            .map(|message| LodestoneAppMessage::Instance(InstanceMessage::Browser(message)))
    }

    /// Selects the entries at `written` once the folder has been read again, where the folder on screen holds them.
    fn select_written(&mut self, written: &[PathBuf]) {
        let root = self.instance_page.browser().root().to_path_buf();
        let paths: Vec<String> = written.iter().filter_map(|path| relative(&root, path)).collect();
        self.instance_page.browser_mut().select_when_listed(paths);
    }

    /// Puts entries on the clipboard: the launcher remembers them, so a paste here always works, and the system's
    /// clipboard takes them as files, so another app can paste them too.
    fn clip_files(&mut self, root: &Path, paths: &[String], cut: bool) -> Task<LodestoneAppMessage> {
        let paths: Vec<PathBuf> = paths.iter().filter_map(|path| ops::inside(root, path)).collect();
        if paths.is_empty() {
            return Task::none();
        }
        self.instance_page.browser_mut().set_clip(Some(Clip { paths: paths.clone(), cut }));
        let sender = self.clipboard.sender();
        Task::perform(clipboard::copy_files(sender, paths), |result| {
            LodestoneAppMessage::Files(Message::Clipped(result))
        })
    }

    /// Pastes into the folder `into`: the files the system's clipboard holds, or the ones the launcher remembers when
    /// the clipboard could not be read. Entries that were cut here are moved, and anything else is copied.
    fn paste_files(&mut self, into: String, clipboard: Result<Vec<PathBuf>, String>) -> Task<LodestoneAppMessage> {
        let root = self.instance_page.browser().root().to_path_buf();
        let Some(into) = ops::inside(&root, &into) else {
            return Task::none();
        };
        match paste_plan(clipboard.ok(), self.instance_page.browser().clip()) {
            Some((sources, mode)) => self.run_transfer(Transfer { sources, into, mode }, Existing::Report),
            None => Task::none(),
        }
    }

    /// Copies or moves the entries of `job` on the blocking pool. The outcome returns as `Transferred`.
    fn run_transfer(&mut self, job: Transfer, existing: Existing) -> Task<LodestoneAppMessage> {
        let generation = self.instance_page.generation();
        let word = lang::t("instance.files.copy_word");
        let work = job.clone();
        Task::perform(
            async move {
                tokio::task::spawn_blocking(move || ops::transfer(&work, existing, word, files::move_to_trash))
                    .await
                    .unwrap_or_default()
            },
            move |report| {
                LodestoneAppMessage::Files(Message::Transferred {
                    generation,
                    job: job.clone(),
                    report,
                })
            },
        )
    }

    /// Applies a transfer's outcome: the folder is read again with what was written selected, a cut that was pasted is
    /// forgotten, failures are reported, and entries that found a name in their way are asked about.
    fn transferred(&mut self, job: Transfer, report: Report) -> Task<LodestoneAppMessage> {
        if job.mode == Mode::Move {
            let browser = self.instance_page.browser_mut();
            if browser
                .clip()
                .is_some_and(|clip| clip.cut && clip.paths.iter().any(|path| job.sources.contains(path)))
            {
                browser.set_clip(None);
            }
        }
        self.select_written(&report.written);
        // A move may have taken the file open in the editor with it.
        let mut tasks = vec![self.refresh_files(), self.check_open_file()];
        if !report.errors.is_empty() {
            let mut text = lang::t_count("instance.files.transfer.failed", report.errors.len());
            for failure in &report.errors {
                let reason = match &failure.reason {
                    ops::Reason::IntoItself => lang::t("instance.files.transfer.into_itself"),
                    ops::Reason::Io(error) => error.as_str(),
                };
                text.push('\n');
                text.push_str(&lang::t_with(
                    "instance.files.failed_line",
                    &[("name", failure.name.as_str()), ("error", reason)],
                ));
            }
            tasks.push(self.alerts.show(Alert::error(text)).discard());
        }
        if !report.conflicts.is_empty() {
            tasks.push(self.ask_replace(job, report.conflicts));
        }
        Task::batch(tasks)
    }

    /// Asks whether the entries at `conflicts` replace the ones the target folder already holds under their names. The
    /// replaced entries go to the trash.
    fn ask_replace(&mut self, job: Transfer, conflicts: Vec<PathBuf>) -> Task<LodestoneAppMessage> {
        let generation = self.instance_page.generation();
        let folder = name_of(&job.into);
        let message = match conflicts.as_slice() {
            [only] => lang::t_with(
                "instance.files.replace.message_one",
                &[("name", name_of(only).as_str()), ("folder", folder.as_str())],
            ),
            _ => lang::t_with(
                "instance.files.replace.message_many",
                &[("count", conflicts.len().to_string().as_str()), ("folder", folder.as_str())],
            ),
        };
        let job = Transfer { sources: conflicts, ..job };
        self.alerts
            .show(
                Alert::warn(message)
                    .title(lang::t("instance.files.replace.title"))
                    .labelled(lang::t("instance.files.replace.confirm"), Some(lang::t("instance.files.replace.skip"))),
            )
            .map(move |confirmed| {
                LodestoneAppMessage::Files(Message::ReplaceAnswered {
                    generation,
                    job: job.clone(),
                    confirmed,
                })
            })
    }

    /// Asks before entries go to the trash. The answer returns as `DeleteAnswered`.
    fn ask_file_delete(&mut self, paths: Vec<String>) -> Task<LodestoneAppMessage> {
        if paths.is_empty() {
            return Task::none();
        }
        let generation = self.instance_page.generation();
        let message = match paths.as_slice() {
            [only] => lang::t_with("instance.files.delete.message_one", &[("name", name_of(Path::new(only)).as_str())]),
            _ => lang::t_with("instance.files.delete.message_many", &[("count", paths.len().to_string().as_str())]),
        };
        self.alerts
            .show(Alert::danger(message).title(lang::t("instance.files.delete.title")))
            .map(move |confirmed| {
                LodestoneAppMessage::Files(Message::DeleteAnswered {
                    generation,
                    paths: paths.clone(),
                    confirmed,
                })
            })
    }

    /// Moves the entries to the trash once the user has said yes. The outcome returns as `Deleted`.
    fn answer_file_delete(&mut self, paths: Vec<String>, confirmed: bool) -> Task<LodestoneAppMessage> {
        if !confirmed {
            return Task::none();
        }
        let root = self.instance_page.browser().root().to_path_buf();
        let generation = self.instance_page.generation();
        let entries: Vec<PathBuf> = paths
            .iter()
            .filter(|path| !path.is_empty())
            .filter_map(|path| ops::inside(&root, path))
            .collect();
        Task::perform(
            async move {
                tokio::task::spawn_blocking(move || {
                    entries
                        .iter()
                        .filter_map(|entry| files::move_to_trash(entry).err().map(|error| (name_of(entry), error)))
                        .collect::<Vec<_>>()
                })
                .await
                .unwrap_or_default()
            },
            move |failed| LodestoneAppMessage::Files(Message::Deleted { generation, failed }),
        )
    }
}

#[cfg(test)]
#[path = "../../../tests/app/file_actions/mod.rs"]
mod tests;
