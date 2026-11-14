//! The launcher picker: the instances the other launchers on this machine keep, which the user checks to copy into Lodestone. The
//! Import dialog's From launcher tab and the setup wizard's Import step both show it, and both read the checked instances
//! through `work`. The import itself is the same for both (see `import/mod.rs`).

mod view;

use std::collections::BTreeSet;
use std::path::PathBuf;

use iced::{Element, Task};

use super::launcher::{self, Launcher, Pick, Scan};
use super::prepare::Work;
use crate::app::lang;

/// The picker's messages: looking for the launchers, locating one by hand, and checking the instances.
#[derive(Debug, Clone)]
pub enum Message {
    /// The launchers are looked for, in the background. The picker asks for this when it is first shown.
    Scan,
    /// The launchers were looked for, with what each folder held.
    Scanned(Vec<Scan>),
    /// The Locate button of a launcher was pressed: the folder picker opens.
    Locate(Launcher),
    /// The folder picker answered for a launcher, or was cancelled.
    LocatePicked(Launcher, Option<PathBuf>),
    /// The folder a launcher was located in was checked, and listed when it holds the launcher's instances.
    Located(Launcher, Result<Scan, String>),
    /// An instance's checkbox was pressed.
    ToggleInstance(Pick),
    /// A launcher's select-all checkbox was pressed.
    ToggleLauncher(Launcher),
}

/// What the picker holds: what each launcher's folder held, whether the launchers are being looked for, and the instances checked.
#[derive(Debug, Clone, Default)]
pub struct LauncherPicker {
    /// What each launcher's folder held, in the order the launchers are listed. Empty until the launchers are looked for.
    pub scans: Vec<Scan>,
    /// Whether the launchers are being looked for.
    pub scanning: bool,
    /// The instances the user has checked.
    pub chosen: BTreeSet<Pick>,
}

impl LauncherPicker {
    /// Whether an instance is checked.
    pub fn has_chosen(&self) -> bool {
        !self.chosen.is_empty()
    }

    /// How many instances are checked.
    pub fn chosen_count(&self) -> usize {
        self.chosen.len()
    }

    /// Forgets the checked instances, once they have been imported.
    pub fn clear_chosen(&mut self) {
        self.chosen.clear();
    }

    /// The import of the checked instances, or `None` when none is checked.
    pub fn work(&self) -> Option<Work> {
        self.has_chosen().then(|| Work::Launchers(self.chosen.iter().cloned().collect()))
    }

    /// Applies a message. Looking for the launchers and locating one are tasks, which answer with messages of this type.
    pub fn update(&mut self, message: Message) -> Task<Message> {
        match message {
            Message::Scan => return self.scan(),
            Message::Scanned(scans) => {
                self.scanning = false;
                self.scans = scans;
            }
            Message::Locate(launcher) => {
                return Task::perform(pick_launcher_folder(), move |folder| Message::LocatePicked(launcher, folder));
            }
            Message::LocatePicked(launcher, Some(folder)) => {
                return Task::perform(
                    async move {
                        tokio::task::spawn_blocking(move || launcher::locate(launcher, folder))
                            .await
                            .unwrap_or_else(|error| Err(error.to_string()))
                    },
                    move |result| Message::Located(launcher, result),
                );
            }
            Message::LocatePicked(_, None) => {}
            Message::Located(launcher, Ok(scan)) => {
                self.chosen.retain(|pick| pick.launcher != launcher);
                if let Some(slot) = self.scans.iter_mut().find(|slot| slot.launcher == launcher) {
                    *slot = scan;
                }
            }
            Message::Located(launcher, Err(error)) => {
                if let Some(slot) = self.scans.iter_mut().find(|slot| slot.launcher == launcher) {
                    slot.error = Some(error);
                }
            }
            Message::ToggleInstance(pick) => {
                if self.is_available(&pick) && !self.chosen.remove(&pick) {
                    self.chosen.insert(pick);
                }
            }
            Message::ToggleLauncher(launcher) => {
                let available = self.available(launcher);
                if available.iter().all(|pick| self.chosen.contains(pick)) {
                    for pick in &available {
                        self.chosen.remove(pick);
                    }
                } else {
                    self.chosen.extend(available);
                }
            }
        }
        Task::none()
    }

    /// The rows the picker shows: a section per launcher, with a checkbox for each instance. The Locate buttons are hidden while an
    /// import reads.
    pub fn view(&self, busy: bool) -> Element<'static, Message> {
        view::rows(&self.scans, &self.chosen, self.scanning, busy)
    }

    /// Looks for the launchers on this machine in the background. The answer comes back as `Scanned`.
    fn scan(&mut self) -> Task<Message> {
        self.scanning = true;
        Task::perform(
            async { tokio::task::spawn_blocking(launcher::scan_all).await.unwrap_or_default() },
            Message::Scanned,
        )
    }

    /// Whether `pick` names an instance the launchers listed, and that can be imported.
    fn is_available(&self, pick: &Pick) -> bool {
        self.scans.iter().any(|scan| {
            scan.launcher == pick.launcher
                && scan
                    .instances
                    .iter()
                    .any(|instance| instance.source == pick.source && instance.summary.is_ok())
        })
    }

    /// Every instance of `launcher` that can be imported.
    fn available(&self, launcher: Launcher) -> Vec<Pick> {
        self.scans
            .iter()
            .filter(|scan| scan.launcher == launcher)
            .flat_map(|scan| scan.instances.iter())
            .filter(|instance| instance.summary.is_ok())
            .map(|instance| Pick {
                launcher,
                source: instance.source.clone(),
            })
            .collect()
    }
}

/// Asks for the folder a launcher keeps its instances in. `None` when it is cancelled.
async fn pick_launcher_folder() -> Option<PathBuf> {
    rfd::AsyncFileDialog::new()
        .set_title(lang::t("import.launcher.locate_title"))
        .pick_folder()
        .await
        .map(|folder| folder.path().to_path_buf())
}

#[cfg(test)]
#[path = "../../../tests/app/import/launcher_picker.rs"]
mod tests;
