//! The folder the launcher keeps its data in: the instances, the Java runtimes, the assets, the libraries and the skins. The
//! Settings page moves that data to a folder the user picks, and this module carries the move out.
//!
//! A move runs on the blocking pool while the window is covered, so no game or install works on the folder mid-move. Each
//! top-level entry of the old folder moves on its own: a rename when the new folder is on the same drive, and a copy then
//! a removal when it is not. A move that fails puts the entries back, so the data stays in one place. The choice is saved
//! in the preferences, which stay in the config folder, only once the data has landed.

use std::ffi::OsString;
use std::fs;
use std::io;
use std::path::{Path, PathBuf};

use iced::{Element, Task};
use tracing::{info, warn};

use super::components::alert::Alert;
use super::components::modal::Modal;
use super::lang;
use super::pages::settings::display_path;
use super::paths::{self, LauncherPaths};
use super::preferences::Preferences;
use super::{LodestoneAppMessage, LodestoneLauncher};

/// Why a folder cannot take the data.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Refusal {
    /// The new folder is inside the current one, or holds it.
    Nested,
    /// The new folder already has files in it.
    NotEmpty,
}

impl Refusal {
    /// The sentence the Settings page shows for this refusal.
    fn text(self) -> &'static str {
        match self {
            Refusal::Nested => lang::t("data_folder.refused.nested"),
            Refusal::NotEmpty => lang::t("data_folder.refused.not_empty"),
        }
    }
}

/// Checks that `to` can take the data held in `from`: it is not nested with `from`, and it is empty or does not exist yet.
pub fn check_target(from: &Path, to: &Path) -> Result<(), Refusal> {
    if to.starts_with(from) || from.starts_with(to) {
        return Err(Refusal::Nested);
    }
    if to.exists() && !is_empty_dir(to) {
        return Err(Refusal::NotEmpty);
    }
    Ok(())
}

fn is_empty_dir(dir: &Path) -> bool {
    dir.is_dir() && fs::read_dir(dir).is_ok_and(|mut entries| entries.next().is_none())
}

/// Moves every top-level entry of `from` into `to`, then removes `from` once it is empty. When an entry cannot move, the
/// entries that did are put back, and the error says so.
fn move_root(from: &Path, to: &Path) -> Result<(), String> {
    check_target(from, to).map_err(|refusal| format!("the data cannot move: {refusal:?}"))?;
    fs::create_dir_all(to).map_err(|error| format!("creating {}: {error}", to.display()))?;
    let mut moved: Vec<OsString> = Vec::new();
    for name in entry_names(from)? {
        if let Err(error) = move_entry(&from.join(&name), &to.join(&name)) {
            let stuck = restore(from, to, &moved);
            return Err(if stuck.is_empty() {
                error
            } else {
                format!("{error}; {} entries could not move back from {}", stuck.len(), to.display())
            });
        }
        moved.push(name);
    }
    // The old folder is empty now. One that cannot be removed stays, empty, which does no harm.
    let _ = fs::remove_dir(from);
    Ok(())
}

/// The names of the entries in `dir`. A folder that does not exist yet holds none.
fn entry_names(dir: &Path) -> Result<Vec<OsString>, String> {
    match fs::read_dir(dir) {
        Ok(entries) => entries
            .map(|entry| entry.map(|entry| entry.file_name()))
            .collect::<io::Result<Vec<_>>>()
            .map_err(|error| format!("reading {}: {error}", dir.display())),
        Err(error) if error.kind() == io::ErrorKind::NotFound => Ok(Vec::new()),
        Err(error) => Err(format!("reading {}: {error}", dir.display())),
    }
}

/// Moves one entry to `target`, which must not exist. A rename does it when both are on one drive.
fn move_entry(source: &Path, target: &Path) -> Result<(), String> {
    match fs::rename(source, target) {
        Ok(()) => Ok(()),
        Err(error) if error.kind() == io::ErrorKind::CrossesDevices => copy_then_remove(source, target),
        Err(error) => Err(format!("moving {}: {error}", source.display())),
    }
}

/// Copies `source` to `target`, then removes `source`. A copy that fails is removed again, so the source is left whole.
fn copy_then_remove(source: &Path, target: &Path) -> Result<(), String> {
    if let Err(error) = copy_tree(source, target) {
        let _ = remove_tree(target);
        return Err(format!("copying {} to {}: {error}", source.display(), target.display()));
    }
    remove_tree(source).map_err(|error| format!("removing {} after its copy: {error}", source.display()))
}

/// Copies a file, a folder and everything in it, keeping links as links. A link is not followed, so a link to a folder
/// above it cannot copy the same files forever.
fn copy_tree(source: &Path, target: &Path) -> io::Result<()> {
    let kind = fs::symlink_metadata(source)?.file_type();
    if kind.is_symlink() {
        return copy_link(source, target);
    }
    if kind.is_dir() {
        fs::create_dir(target)?;
        for entry in fs::read_dir(source)? {
            let entry = entry?;
            copy_tree(&entry.path(), &target.join(entry.file_name()))?;
        }
        return Ok(());
    }
    fs::copy(source, target).map(|_| ())
}

#[cfg(unix)]
fn copy_link(source: &Path, target: &Path) -> io::Result<()> {
    std::os::unix::fs::symlink(fs::read_link(source)?, target)
}

#[cfg(not(unix))]
fn copy_link(source: &Path, target: &Path) -> io::Result<()> {
    fs::copy(source, target).map(|_| ())
}

/// Removes a file, a link, or a folder with everything in it. A link is removed as a link.
fn remove_tree(path: &Path) -> io::Result<()> {
    if fs::symlink_metadata(path)?.file_type().is_dir() {
        fs::remove_dir_all(path)
    } else {
        fs::remove_file(path)
    }
}

/// Moves the entries that moved back into `from`, newest first. Returns the names that could not move back.
fn restore(from: &Path, to: &Path, moved: &[OsString]) -> Vec<OsString> {
    let mut stuck = Vec::new();
    for name in moved.iter().rev() {
        if let Err(error) = move_entry(&to.join(name), &from.join(name)) {
            warn!("{error}");
            stuck.push(name.clone());
        }
    }
    stuck
}

/// Moves the data from `from` to `to`, then saves the choice: `saved` is the folder the launcher opens next time, or
/// `None` for the default. When the save fails, the data moves back, so the launcher and its data still agree. Runs on the
/// blocking pool.
pub fn apply(from: &Path, to: &Path, saved: Option<PathBuf>) -> Result<(), String> {
    move_root(from, to)?;
    if let Err(error) = save(saved) {
        warn!("Saving the data folder failed, so the data moves back: {error}");
        return Err(match move_root(to, from) {
            Ok(()) => error,
            Err(back) => format!("{error}; the data could not move back: {back}"),
        });
    }
    Ok(())
}

/// Saves the folder the launcher opens next time. The other preferences are kept as the file has them.
fn save(folder: Option<PathBuf>) -> Result<(), String> {
    let mut preferences = Preferences::load();
    preferences.data_folder = folder;
    preferences.store()
}

impl LodestoneLauncher {
    /// Why the data cannot move now: a game is running, or an install, a mod job or an export has not ended.
    fn data_busy(&self) -> Option<&'static str> {
        if self.games.any_active() {
            Some(lang::t("data_folder.busy.games"))
        } else if !self.downloads.pending().is_empty() {
            Some(lang::t("data_folder.busy.downloads"))
        } else {
            None
        }
    }

    /// Asks for the folder the data should move to. It is refused while the data is busy.
    pub(super) fn choose_data_folder(&mut self) -> Task<LodestoneAppMessage> {
        if let Some(reason) = self.data_busy() {
            return self.alerts.show(Alert::error(reason)).discard();
        }
        Task::perform(pick_folder(), LodestoneAppMessage::DataFolderPicked)
    }

    /// Moves the data back to the default folder, after the same confirmation as a move to a folder the user picked.
    pub(super) fn use_default_data_folder(&mut self) -> Task<LodestoneAppMessage> {
        self.ask_data_move(paths::default_data_root())
    }

    /// The folder picker answered. A cancel does nothing.
    pub(super) fn data_folder_picked(&mut self, folder: Option<PathBuf>) -> Task<LodestoneAppMessage> {
        match folder {
            Some(folder) => self.ask_data_move(folder),
            None => Task::none(),
        }
    }

    /// Checks the folder, then asks the user to confirm the move. A folder that cannot take the data is explained instead.
    fn ask_data_move(&mut self, folder: PathBuf) -> Task<LodestoneAppMessage> {
        if let Some(reason) = self.data_busy() {
            return self.alerts.show(Alert::error(reason)).discard();
        }
        let from = self.paths.root();
        if folder == from {
            return Task::none();
        }
        if let Err(refusal) = check_target(&from, &folder) {
            return self.alerts.show(Alert::error(refusal.text())).discard();
        }
        let body = lang::t_with(
            "data_folder.confirm.body",
            &[("from", display_path(&from).as_str()), ("to", display_path(&folder).as_str())],
        );
        self.alerts
            .show(Alert::warn(body).title(lang::t("data_folder.confirm.title")).yes_no())
            .map(move |confirmed| LodestoneAppMessage::DataFolderAnswered {
                folder: folder.clone(),
                confirmed,
            })
    }

    /// The answer to the question. A yes starts the move, which covers the window until it lands.
    pub(super) fn data_folder_answered(&mut self, folder: PathBuf, confirmed: bool) -> Task<LodestoneAppMessage> {
        if !confirmed || self.data_move.is_some() {
            return Task::none();
        }
        // A game or an install may have started while the question was open, so the data is checked again.
        if let Some(reason) = self.data_busy() {
            return self.alerts.show(Alert::error(reason)).discard();
        }
        let from = self.paths.root();
        let saved = (folder != paths::default_data_root()).then(|| folder.clone());
        info!("Moving the launcher's data from {} to {}", from.display(), folder.display());
        self.data_move = Some(folder.clone());
        let to = folder.clone();
        Task::perform(
            async move {
                match tokio::task::spawn_blocking(move || apply(&from, &to, saved)).await {
                    Ok(result) => result,
                    Err(error) => Err(error.to_string()),
                }
            },
            move |result| LodestoneAppMessage::DataFolderMoved { folder, result },
        )
    }

    /// The move finished. On success the launcher works from the new folder: the library reads it again, and the skins
    /// are scanned there. A failure is shown, and the data stays where it was.
    pub(super) fn data_folder_moved(&mut self, folder: PathBuf, result: Result<(), String>) -> Task<LodestoneAppMessage> {
        self.data_move = None;
        match result {
            Ok(()) => {
                info!("The launcher's data is in {}", folder.display());
                self.paths = LauncherPaths::new(folder);
                self.settings.set_paths(&self.paths);
                Task::batch([self.load_instances(), self.scan_task()])
            }
            Err(error) => {
                warn!("Moving the launcher's data failed: {error}");
                self.alerts
                    .show(Alert::error(lang::t_with("data_folder.failed", &[("error", error.as_str())])))
                    .discard()
            }
        }
    }
}

/// The window's cover while the data moves. It takes every event, so nothing runs on the folder until the move lands.
pub fn moving_cover<'a>(to: &Path) -> Element<'a, LodestoneAppMessage> {
    Modal::new(
        lang::t("data_folder.moving.title"),
        lang::t_with("data_folder.moving.body", &[("to", display_path(to).as_str())]),
    )
    .close_on_scrim(false)
    .into()
}

/// Asks for a folder to keep the data in. `None` when it is cancelled.
async fn pick_folder() -> Option<PathBuf> {
    rfd::AsyncFileDialog::new()
        .set_title(lang::t("data_folder.pick_title"))
        .pick_folder()
        .await
        .map(|folder| folder.path().to_path_buf())
}

#[cfg(test)]
#[path = "../../tests/app/data_folder.rs"]
mod tests;
