//! The folders the launcher keeps its files in, and how the user changes them. The instances and the shared data (the data folder:
//! the Java runtimes, the libraries and assets, the Discover downloads and the skins) move with their contents. A move runs on
//! the blocking pool while the window is covered, so no game or install works on the folder mid-move. Each top-level entry of
//! the old folder moves on its own: a rename when the new folder is on the same drive, and a copy then a removal when it is not.
//! A move that fails puts the entries back, so the data stays in one place. The choice is saved in the preferences, which stay in
//! the config folder, only once the data has landed. The logs and the cache hold nothing a move needs, so their new folder is
//! used from the next start instead.

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

/// One of the folders the launcher keeps its files in.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum FolderKind {
    /// Every instance's game files, mods, worlds and screenshots.
    Instances,
    /// The data folder: the Java runtimes, the libraries and assets, the Discover downloads and the skins.
    Shared,
    /// The logs the launcher and the games write.
    Logs,
    /// The caches the launcher can fetch again: the news, the pictures and the staged updates.
    Cache,
    /// The sign-ins, the database and the preferences. It stays where it is.
    Config,
}

impl FolderKind {
    /// Whether a change moves the files in the folder, so the move waits until nothing uses them.
    pub fn moves_data(self) -> bool {
        matches!(self, FolderKind::Instances | FolderKind::Shared)
    }
}

/// What a folder row's button does.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum FolderAction {
    /// Opens the folder in the file manager.
    Open,
    /// Asks for another folder.
    Change,
    /// Goes back to the default folder.
    Reset,
}

/// One row of the folders list: the folder, where it is, and which of its buttons apply.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Row {
    pub kind: FolderKind,
    /// The folder the row names: the one in use, or the one the launcher uses from the next start.
    pub path: PathBuf,
    /// Whether the folder is the default one, so the row offers no Default button.
    pub default: bool,
    /// Whether the folder can be changed. A portable launcher's and a development run's folders are fixed, and the config
    /// folder never moves.
    pub editable: bool,
}

/// The rows of the launcher's folders, in the order the lists show them.
pub fn rows(paths: &LauncherPaths) -> Vec<Row> {
    let fixed = paths::data_folder_fixed();
    vec![
        Row {
            kind: FolderKind::Instances,
            path: paths.instances(),
            default: paths.instances_chosen().is_none(),
            editable: !fixed,
        },
        Row {
            kind: FolderKind::Shared,
            path: paths.root(),
            default: paths.root() == paths::default_data_root(),
            editable: !fixed,
        },
        Row {
            kind: FolderKind::Logs,
            path: paths.logs(),
            default: paths.logs_chosen().is_none(),
            editable: !fixed,
        },
        Row {
            kind: FolderKind::Cache,
            path: paths.cache(),
            default: paths.cache_chosen().is_none(),
            editable: !fixed,
        },
        Row {
            kind: FolderKind::Config,
            path: paths::config_dir().unwrap_or_default(),
            default: true,
            editable: false,
        },
    ]
}

/// Why a folder cannot take the files.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Refusal {
    /// The new folder is inside the current one, or holds it.
    Nested,
    /// The new folder already has files in it.
    NotEmpty,
    /// The instances would sit inside the data folder, which moves with its contents and would leave their saved paths behind.
    InsideData,
}

impl Refusal {
    /// The sentence the folder rows show for this refusal.
    fn text(self) -> &'static str {
        match self {
            Refusal::Nested => lang::t("data_folder.refused.nested"),
            Refusal::NotEmpty => lang::t("data_folder.refused.not_empty"),
            Refusal::InsideData => lang::t("data_folder.refused.inside_data"),
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

/// Checks that the instances in `from` can move to `to`. The instances may not sit inside the data folder `root`, which moves
/// with its contents, unless they are the default folder there. A folder that holds the data cannot take them either.
pub fn check_instances_target(from: &Path, root: &Path, to: &Path) -> Result<(), Refusal> {
    check_target(from, to)?;
    if to.starts_with(root) && to != root.join("instances") {
        return Err(Refusal::InsideData);
    }
    if root.starts_with(to) {
        return Err(Refusal::Nested);
    }
    Ok(())
}

/// Checks that the data folder `from` can move to `to`. It may not nest with the instances' folder `instances` either: their
/// saved paths are absolute, so a data folder that holds them, or that they hold, would leave those paths behind.
pub fn check_shared_target(from: &Path, instances: &Path, to: &Path) -> Result<(), Refusal> {
    check_target(from, to)?;
    if to.starts_with(instances) || instances.starts_with(to) {
        return Err(Refusal::Nested);
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

/// Moves the data folder from `from` to `to`, then saves the choice: `saved` is the folder the launcher opens next time, or
/// `None` for the default. When the save fails, the data moves back, so the launcher and its data still agree. Runs on the
/// blocking pool.
pub fn apply(from: &Path, to: &Path, saved: Option<PathBuf>) -> Result<(), String> {
    move_then_save(from, to, saved, |folder| {
        Preferences::update(|preferences| preferences.data_folder = folder)
    })
}

/// Moves the instances from `from` to `to`, then saves the choice as `apply` does. Runs on the blocking pool.
pub fn apply_instances(from: &Path, to: &Path, saved: Option<PathBuf>) -> Result<(), String> {
    move_then_save(from, to, saved, |folder| {
        Preferences::update(|preferences| preferences.instances_folder = folder)
    })
}

/// Moves `from` to `to`, then runs `save` with `saved`. When `save` fails, the entries move back.
fn move_then_save(from: &Path, to: &Path, saved: Option<PathBuf>, save: impl FnOnce(Option<PathBuf>) -> Result<(), String>) -> Result<(), String> {
    move_root(from, to)?;
    if let Err(error) = save(saved) {
        warn!("Saving the folder choice failed, so its files move back: {error}");
        return Err(match move_root(to, from) {
            Ok(()) => error,
            Err(back) => format!("{error}; the files could not move back: {back}"),
        });
    }
    Ok(())
}

/// Saves the folder the logs or the cache go to from the next start, `None` for the default. Runs on the blocking pool.
fn save_choice(kind: FolderKind, folder: Option<PathBuf>) -> Result<(), String> {
    Preferences::update(|preferences| match kind {
        FolderKind::Logs => preferences.logs_folder = folder,
        FolderKind::Cache => preferences.cache_folder = folder,
        FolderKind::Instances | FolderKind::Shared | FolderKind::Config => {}
    })
}

impl LodestoneLauncher {
    /// Why the data cannot move now: a game is running, or an install, a mod job or an export has not ended.
    fn data_busy(&self) -> Option<&'static str> {
        if self.games.any_active() {
            Some(lang::t("data_folder.busy.games"))
        } else if self.downloads.has_unfinished() {
            Some(lang::t("data_folder.busy.downloads"))
        } else {
            None
        }
    }

    /// The folder a row names now.
    fn folder_path(&self, kind: FolderKind) -> PathBuf {
        match kind {
            FolderKind::Instances => self.paths.instances(),
            FolderKind::Shared => self.paths.root(),
            FolderKind::Logs => self.paths.logs(),
            FolderKind::Cache => self.paths.cache(),
            FolderKind::Config => paths::config_dir().unwrap_or_default(),
        }
    }

    /// The default folder of a row, which its Default button goes back to.
    fn default_folder(&self, kind: FolderKind) -> PathBuf {
        match kind {
            FolderKind::Instances => self.paths.root().join("instances"),
            FolderKind::Shared => paths::default_data_root(),
            FolderKind::Logs => paths::default_logs_dir().unwrap_or_default(),
            FolderKind::Cache => paths::default_cache_dir().unwrap_or_default(),
            FolderKind::Config => paths::config_dir().unwrap_or_default(),
        }
    }

    /// A folder row's button was pressed: the folder opens, or it is changed, or it goes back to its default.
    pub(super) fn folder_message(&mut self, kind: FolderKind, action: FolderAction) -> Task<LodestoneAppMessage> {
        match action {
            FolderAction::Open => super::instances::reveal_folder(self.folder_path(kind)),
            FolderAction::Change => self.choose_folder(kind),
            FolderAction::Reset => self.reset_folder(kind),
        }
    }

    /// Asks for the folder a row should move to, or keep its logs or cache in. A portable launcher's and a development run's
    /// folders are fixed, and the config folder never moves, so these ask for nothing.
    fn choose_folder(&mut self, kind: FolderKind) -> Task<LodestoneAppMessage> {
        if paths::data_folder_fixed() || kind == FolderKind::Config {
            return Task::none();
        }
        if kind.moves_data()
            && let Some(reason) = self.data_busy()
        {
            return self.alerts.show(Alert::error(reason)).discard();
        }
        Task::perform(pick_folder(), move |folder| LodestoneAppMessage::FolderPicked { kind, folder })
    }

    /// Sends a row back to its default folder: the files move back when the row moves its files, after the same confirmation as
    /// a folder picked, and the logs and cache use the default from the next start.
    fn reset_folder(&mut self, kind: FolderKind) -> Task<LodestoneAppMessage> {
        if paths::data_folder_fixed() || kind == FolderKind::Config {
            return Task::none();
        }
        if kind.moves_data() {
            let folder = self.default_folder(kind);
            return self.ask_move(kind, folder);
        }
        self.save_choice_now(kind, None)
    }

    /// The folder picker answered for `kind`. A cancel does nothing.
    pub(super) fn folder_picked(&mut self, kind: FolderKind, folder: Option<PathBuf>) -> Task<LodestoneAppMessage> {
        let Some(folder) = folder else {
            return Task::none();
        };
        if kind.moves_data() {
            self.ask_move(kind, folder)
        } else {
            self.save_choice_now(kind, Some(folder))
        }
    }

    /// Checks the folder the files of `kind` would move to, then asks the user to confirm the move. A folder that cannot take the
    /// files is explained instead.
    fn ask_move(&mut self, kind: FolderKind, folder: PathBuf) -> Task<LodestoneAppMessage> {
        if paths::data_folder_fixed() {
            return Task::none();
        }
        if let Some(reason) = self.data_busy() {
            return self.alerts.show(Alert::error(reason)).discard();
        }
        let from = self.folder_path(kind);
        if folder == from {
            return Task::none();
        }
        let checked = match kind {
            FolderKind::Instances => check_instances_target(&from, &self.paths.root(), &folder),
            _ => check_shared_target(&from, &self.paths.instances(), &folder),
        };
        if let Err(refusal) = checked {
            return self.alerts.show(Alert::error(refusal.text())).discard();
        }
        let body_key = match kind {
            FolderKind::Instances => "folders.instances.confirm_body",
            _ => "data_folder.confirm.body",
        };
        let body = lang::t_with(
            body_key,
            &[("from", display_path(&from).as_str()), ("to", display_path(&folder).as_str())],
        );
        self.alerts
            .show(Alert::warn(body).title(lang::t("data_folder.confirm.title")).yes_no())
            .map(move |confirmed| LodestoneAppMessage::FolderAnswered {
                kind,
                folder: folder.clone(),
                confirmed,
            })
    }

    /// The answer to the question. A yes starts the move, which covers the window until it lands.
    pub(super) fn folder_answered(&mut self, kind: FolderKind, folder: PathBuf, confirmed: bool) -> Task<LodestoneAppMessage> {
        if !confirmed || self.data_move.is_some() || paths::data_folder_fixed() {
            return Task::none();
        }
        // A game or an install may have started while the question was open, so the data is checked again.
        if let Some(reason) = self.data_busy() {
            return self.alerts.show(Alert::error(reason)).discard();
        }
        let from = self.folder_path(kind);
        // The folder the launcher opens next time, or `None` for the default.
        let saved = (folder != self.default_folder(kind)).then(|| folder.clone());
        info!("Moving the launcher's {kind:?} from {} to {}", from.display(), folder.display());
        self.data_move = Some(folder.clone());
        let to = folder.clone();
        Task::perform(
            async move {
                let result = tokio::task::spawn_blocking(move || match kind {
                    FolderKind::Instances => apply_instances(&from, &to, saved),
                    _ => apply(&from, &to, saved),
                })
                .await;
                match result {
                    Ok(result) => result,
                    Err(error) => Err(error.to_string()),
                }
            },
            move |result| LodestoneAppMessage::FolderMoved { kind, folder, result },
        )
    }

    /// The files moved to `folder`, or the move failed and they stayed where they were. On success the launcher works from the
    /// new folder: the library reads the instances again, and the skins are scanned there.
    pub(super) fn folder_moved(&mut self, kind: FolderKind, folder: PathBuf, result: Result<(), String>) -> Task<LodestoneAppMessage> {
        self.data_move = None;
        match result {
            Ok(()) => {
                info!("The launcher's {kind:?} are in {}", folder.display());
                self.paths = match kind {
                    FolderKind::Instances => {
                        let default = self.default_folder(kind);
                        self.paths.clone().with_instances((folder != default).then_some(folder))
                    }
                    _ => self.paths.with_root(folder),
                };
                self.settings.set_paths(&self.paths);
                Task::batch([self.load_instances(), self.scan_task()])
            }
            Err(error) => {
                warn!("Moving the launcher's {kind:?} failed: {error}");
                self.alerts
                    .show(Alert::error(lang::t_with("data_folder.failed", &[("error", error.as_str())])))
                    .discard()
            }
        }
    }

    /// Saves the folder the logs or the cache go to from the next start. The rows show the choice at once, and an alert says
    /// when it takes effect.
    fn save_choice_now(&mut self, kind: FolderKind, folder: Option<PathBuf>) -> Task<LodestoneAppMessage> {
        self.paths = match kind {
            FolderKind::Logs => self.paths.clone().with_logs(folder.clone()),
            FolderKind::Cache => self.paths.clone().with_cache(folder.clone()),
            FolderKind::Instances | FolderKind::Shared | FolderKind::Config => self.paths.clone(),
        };
        self.settings.set_paths(&self.paths);
        let save = Task::future(async move {
            match tokio::task::spawn_blocking(move || save_choice(kind, folder)).await {
                Ok(Ok(())) => {}
                Ok(Err(error)) => warn!("Saving the {kind:?} folder failed: {error}"),
                Err(error) => warn!("Saving the {kind:?} folder failed: {error}"),
            }
        })
        .discard();
        let notice = self.alerts.show(Alert::info(lang::t("folders.restart_note"))).discard();
        Task::batch([save, notice])
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

/// Asks for a folder to keep files in. `None` when it is cancelled.
pub async fn pick_folder() -> Option<PathBuf> {
    rfd::AsyncFileDialog::new()
        .set_title(lang::t("data_folder.pick_title"))
        .pick_folder()
        .await
        .map(|folder| folder.path().to_path_buf())
}

#[cfg(test)]
#[path = "../../tests/app/data_folder.rs"]
mod tests;
