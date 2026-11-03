//! The Import Instance dialog: the tab it has open, the file or address it holds, the instances of other launchers it has
//! checked, the recent imports it lists, and the modal that shows them. The root reads what the dialog asks for and queues
//! the installs (see `import/mod.rs`).

mod launchers;
mod recent;
mod zone;

use std::collections::BTreeSet;
use std::path::PathBuf;

use aurora_leaf::Icon;
use iced::task::Handle;
use iced::widget::{Space, Text, column, row, text_input};
use iced::{Element, Fill, Padding, Task, alignment};
use tracing::warn;

use super::Message;
use super::launcher::{self, Launcher, Pick, Scan};
use super::prepare::{self, Picked, Work};
use super::record::{ImportFormat, ImportRecord};
use crate::app::components::button::{TextButton, Variant};
use crate::app::components::input;
use crate::app::components::modal::Modal;
use crate::app::components::segmented::segmented;
use crate::app::lang;
use crate::app::theme::{color, fonts};

/// The modal's width, as the export dialog's.
const WIDTH: f32 = 740.0;
/// The padding of the address field, as the export dialog's fields.
const FIELD_PADDING: Padding = Padding {
    top: 11.0,
    right: 14.0,
    bottom: 11.0,
    left: 14.0,
};

/// The tabs the dialog offers.
#[derive(Debug, Default, Clone, Copy, PartialEq, Eq)]
pub enum Tab {
    /// A modpack file: a `.zip`, a `.mrpack`, or a `.mcpack` read as a zip.
    #[default]
    File,
    /// A CurseForge modpack page, by its address.
    Url,
    /// A Lodestone package, restored as a new instance.
    Backup,
    /// The instances other launchers keep on this machine, which the user checks.
    Launcher,
}

/// The dialog's state.
#[derive(Default)]
pub struct Dialog {
    /// The tab on screen.
    tab: Tab,
    /// The address typed on the URL tab.
    url: String,
    /// The file the file tabs hold, once it is checked.
    picked: Option<Picked>,
    /// Whether a file is being dragged over the window while a file tab is open.
    hovered: bool,
    /// The recent imports, newest first.
    recent: Vec<ImportRecord>,
    /// The handle that stops the read under way. `Some` while a read runs.
    reading: Option<Handle>,
    /// Why the last import did not start. It stays until the next try.
    error: Option<String>,
    /// What each launcher's folder held, in the order the launchers are listed. Empty until the tab is first opened.
    scans: Vec<Scan>,
    /// Whether the launchers are being looked for.
    scanning: bool,
    /// The instances the user has checked on the launcher tab.
    chosen: BTreeSet<Pick>,
}

impl Dialog {
    /// Whether a read is under way. The fields and the zone wait for it.
    pub fn busy(&self) -> bool {
        self.reading.is_some()
    }

    /// Whether Import can be pressed: the tab holds something to import, and no read is under way.
    pub fn ready(&self) -> bool {
        !self.busy()
            && match self.tab {
                Tab::Url => !self.url.trim().is_empty(),
                Tab::File | Tab::Backup => self.picked.is_some(),
                Tab::Launcher => !self.chosen.is_empty(),
            }
    }

    /// The import to read: the recent one at `reimport`, or else the tab's file, address, or checked instances. `None` when
    /// the tab holds nothing to import.
    pub fn work(&self, reimport: Option<usize>) -> Option<Work> {
        if let Some(index) = reimport {
            return self.recent.get(index).cloned().map(Work::Again);
        }
        match self.tab {
            Tab::Url => {
                let url = self.url.trim();
                (!url.is_empty()).then(|| Work::Url(url.to_string()))
            }
            Tab::File => self.picked.as_ref().map(|picked| Work::File(picked.path.clone())),
            Tab::Backup => self.picked.as_ref().map(|picked| Work::Backup(picked.path.clone())),
            Tab::Launcher => (!self.chosen.is_empty()).then(|| Work::Launchers(self.chosen.iter().cloned().collect())),
        }
    }

    /// Records that a read is under way, and the handle that stops it.
    pub fn begin(&mut self, reading: Handle) {
        self.reading = Some(reading);
        self.error = None;
    }

    /// Records why the read did not start, and that none is under way.
    pub fn failed(&mut self, error: String) {
        self.reading = None;
        self.error = Some(error);
    }

    /// Stops the read under way, if there is one.
    pub fn abandon(&mut self) {
        if let Some(reading) = self.reading.take() {
            reading.abort();
        }
    }

    /// Applies a dialog message. Opening the file picker, checking a file, and looking for launchers are tasks here. Closing,
    /// starting an import and its answers belong to the root, since they touch the queue, the database and the library.
    pub fn update(&mut self, message: Message) -> Task<Message> {
        match message {
            Message::TabSelected(tab) => {
                self.tab = tab;
                self.picked = None;
                self.error = None;
                self.hovered = false;
                if tab == Tab::Launcher && self.scans.is_empty() && !self.scanning {
                    return self.scan();
                }
            }
            Message::UrlChanged(url) => self.url = url,
            Message::Hovering(hovering) => self.hovered = hovering && self.takes_files() && !self.busy(),
            Message::Dropped(path) => {
                self.hovered = false;
                if self.takes_files() && !self.busy() {
                    return self.check(path);
                }
            }
            Message::Browse => {
                if self.takes_files() && !self.busy() {
                    return Task::perform(pick(self.tab == Tab::Backup), Message::Picked);
                }
            }
            Message::Picked(Some(path)) => return self.check(path),
            Message::Picked(None) => {}
            Message::Inspected(Ok(picked)) => {
                self.picked = Some(picked);
                self.error = None;
            }
            Message::Inspected(Err(error)) => {
                self.picked = None;
                self.error = Some(error);
            }
            Message::Recent(Ok(recent)) => self.recent = recent,
            Message::Recent(Err(error)) => warn!("reading the recent imports failed: {error}"),
            Message::ScanLaunchers => return self.scan(),
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
            Message::Close | Message::Submit | Message::Reimport(_) | Message::Prepared(_) => {}
        }
        Task::none()
    }

    /// Whether the tab on screen takes files: the file tabs do, the address and launcher tabs do not.
    fn takes_files(&self) -> bool {
        matches!(self.tab, Tab::File | Tab::Backup)
    }

    /// Checks a file for the tab it was given on. Until the check answers, the zone holds nothing.
    fn check(&mut self, path: PathBuf) -> Task<Message> {
        self.picked = None;
        self.error = None;
        Task::perform(prepare::inspect(path, self.tab == Tab::Backup), Message::Inspected)
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

    /// The modal the dialog shows: the tabs, the zone, address or launchers, the status line, the recent imports and the footer.
    pub fn view(&self) -> Element<'_, Message> {
        let body = column![self.tabs(), self.source(), self.status(), recent::view(&self.recent, self.busy())].spacing(18);
        Modal::new(lang::t("import.title"), body)
            .subtitle(lang::t("import.subtitle"))
            .icon(Icon::lucide().download(), color::cyan(), color::cyan_tint())
            .footer(self.footer())
            .on_close(Message::Close)
            .close_on_scrim(true)
            .width(WIDTH)
            .into()
    }

    /// The four tabs, with the one on screen selected.
    fn tabs(&self) -> Element<'_, Message> {
        let options = [
            (Tab::File, lang::t("import.tab.file")),
            (Tab::Url, lang::t("import.tab.url")),
            (Tab::Backup, lang::t("import.tab.backup")),
            (Tab::Launcher, lang::t("import.tab.launcher")),
        ];
        segmented(&options, self.tab, Message::TabSelected)
    }

    /// The drop zone of a file tab, the address field of the URL tab, or the launchers of the launcher tab.
    fn source(&self) -> Element<'_, Message> {
        match self.tab {
            Tab::File | Tab::Backup => zone::view(self.tab == Tab::Backup, self.picked.as_ref(), self.hovered, self.busy()),
            Tab::Url => self.address(),
            Tab::Launcher => launchers::view(&self.scans, &self.chosen, self.scanning, self.busy()),
        }
    }

    /// The address field, with the hint that says what it takes. Enter starts the import, as Import does.
    fn address(&self) -> Element<'_, Message> {
        column![
            Text::new(lang::t("import.url.label"))
                .size(11)
                .font(fonts::JetBrainsMono::REGULAR)
                .color(color::text_muted()),
            text_input(lang::t("import.url.placeholder"), &self.url)
                .on_input(Message::UrlChanged)
                .on_submit(Message::Submit)
                .size(14)
                .padding(FIELD_PADDING)
                .style(input::style)
                .width(Fill),
            Text::new(lang::t("import.url.hint"))
                .size(12)
                .font(fonts::Inter::REGULAR)
                .color(color::text_muted()),
        ]
        .spacing(8)
        .into()
    }

    /// The line under the source: the read under way, or why the last import did not start. Empty otherwise.
    fn status(&self) -> Element<'_, Message> {
        if self.busy() {
            Text::new(lang::t("import.busy"))
                .size(12)
                .font(fonts::Inter::MEDIUM)
                .color(color::text_muted())
                .into()
        } else if let Some(error) = &self.error {
            Text::new(error.as_str()).size(12).font(fonts::Inter::MEDIUM).color(color::pink()).into()
        } else {
            Space::new().height(0.0).into()
        }
    }

    /// The footer: Cancel, and Import, which is lit once there is something to import. On the launcher tab it counts the
    /// checked instances.
    fn footer(&self) -> Element<'_, Message> {
        let label = if self.tab == Tab::Launcher && !self.chosen.is_empty() {
            lang::t_with("import.submit_count", &[("count", self.chosen.len().to_string().as_str())]).to_string()
        } else {
            lang::t("import.submit").to_string()
        };
        let mut submit = TextButton::content(Text::new(label).size(13).font(fonts::Inter::BOLD).color(color::primary_on()))
            .variant(Variant::Cyan)
            .padding(Padding::new(0.0).left(18).right(18).top(9).bottom(9));
        if self.ready() {
            submit = submit.on_press(Message::Submit);
        }
        row![
            Space::new().width(Fill),
            TextButton::content(
                Text::new(lang::t("common.cancel"))
                    .size(13)
                    .font(fonts::Inter::SEMI_BOLD)
                    .color(color::text_default()),
            )
            .variant(Variant::Ghost)
            .padding(Padding::new(0.0).left(18).right(18).top(9).bottom(9))
            .on_press(Message::Close),
            submit,
        ]
        .spacing(10)
        .align_y(alignment::Vertical::Center)
        .into()
    }
}

/// The label of a format, as the zone and the recent imports show it. A launcher's format shows the launcher's name.
pub(super) fn format_label(format: ImportFormat) -> &'static str {
    match format {
        ImportFormat::Mrpack => lang::t("import.format.mrpack"),
        ImportFormat::CurseForge => lang::t("import.format.curseforge"),
        ImportFormat::Lodestone => lang::t("import.format.lodestone"),
        other => Launcher::from_format(other).map_or("", Launcher::name),
    }
}

/// Asks for a file: a `.lodestone` package for the backup tab, a modpack for the file tab. `None` when it is cancelled.
async fn pick(backup: bool) -> Option<PathBuf> {
    let picker = rfd::AsyncFileDialog::new().set_title(lang::t("import.browse.title"));
    let picker = if backup {
        picker.add_filter(lang::t("import.filter.backup"), &["lodestone"])
    } else {
        picker.add_filter(lang::t("import.filter.modpack"), &["zip", "mrpack", "mcpack"])
    };
    picker.pick_file().await.map(|file| file.path().to_path_buf())
}

/// Asks for the folder a launcher is installed in or keeps its instances in. `None` when it is cancelled.
async fn pick_launcher_folder() -> Option<PathBuf> {
    rfd::AsyncFileDialog::new()
        .set_title(lang::t("import.launcher.locate_title"))
        .pick_folder()
        .await
        .map(|folder| folder.path().to_path_buf())
}

#[cfg(test)]
#[path = "../../../../tests/app/import/dialog/mod.rs"]
mod tests;
