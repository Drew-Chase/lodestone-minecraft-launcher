//! The server pack wizard: a full page, opened from an instance's menu, that sorts the instance's mods by side and writes
//! the instance's server pack. The steps are Scan, Review, Export, Exporting and Done. The root starts the scan and the
//! export and reads and writes the choices the database remembers; this page holds what each step shows and the rules
//! that decide which mods go into the pack.

mod done;
mod export;
mod progress;
mod review;

use std::cmp::Ordering;
use std::collections::{HashMap, HashSet};
use std::path::{Path, PathBuf};
use std::sync::Arc;
use std::sync::atomic::AtomicBool;

use iced::Element;
use iced::widget::image;
use lodestone_export::plan::{DEFAULT_EXCLUDED, Selection};
use lodestone_export::server_pack::plan::{self as pack, Choice};
use lodestone_export::server_pack::{ModDetection, Side};
use lodestone_loaders::server::ServerKind;

use crate::app::export::job::file_safe;
use crate::app::lang;

/// The folders a new pack leaves out: the export's own defaults, and the resource and shader packs, which only a
/// client reads.
const LEFT_OUT: [&str; 2] = ["resourcepacks", "shaderpacks"];

/// The labels of the stepper, in order: the Review, Export and Done steps the wizard's stepper shows.
pub fn steps() -> [&'static str; 3] {
    [
        lang::t("server_pack.step.review"),
        lang::t("server_pack.step.export"),
        lang::t("server_pack.step.done"),
    ]
}

/// The steps the wizard goes through, in order.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub enum Step {
    /// The mods are read from the instance's `mods` folder.
    #[default]
    Scanning,
    /// The mods and the folders are reviewed.
    Review,
    /// The output and its destination are chosen.
    Export,
    /// The pack is written.
    Exporting,
    /// The pack is written, and its receipt is shown.
    Done,
}

/// The tab of the Review step.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub enum Tab {
    /// The mods, sorted by side.
    #[default]
    Mods,
    /// The instance's top-level folders, ticked or not.
    Folders,
}

/// The side the mod list shows.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub enum Filter {
    #[default]
    All,
    Client,
    Server,
    Both,
    Unsure,
}

/// The order the mod list shows.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub enum Sort {
    /// Client, server, both and unsure, then by file name.
    #[default]
    Side,
    /// By file name.
    Name,
}

/// What the export writes.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub enum Kind {
    /// A folder that holds the pack's files.
    #[default]
    Folder,
    /// A zip file that holds the pack's files.
    Zip,
}

/// A mod jar the scan read, with its icon decoded for the row.
#[derive(Debug, Clone)]
pub struct ModRow {
    pub detection: ModDetection,
    pub icon: Option<image::Handle>,
}

/// A top-level entry of the instance folder, for the Folders tab.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct FolderRow {
    pub name: String,
    pub is_folder: bool,
}

/// One step of the scan: the jar just read, and its place among the jars. It names the instance folder the scan reads, so
/// a scan that outlives its page cannot change the page that opened after it.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct ScanTick {
    pub folder: PathBuf,
    pub index: usize,
    pub total: usize,
    pub name: String,
    pub side: Option<Side>,
}

/// The end of a scan: the folder it read, and the mods it found or the error it stopped at.
#[derive(Debug, Clone)]
pub struct ScanDone {
    pub folder: PathBuf,
    pub mods: Result<Vec<ModRow>, String>,
}

/// How many jars the scan has placed on each side so far.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub struct Tally {
    pub client: usize,
    pub server: usize,
    pub both: usize,
    pub unsure: usize,
}

/// How far the export has got, in files.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub struct ExportTick {
    pub done: usize,
    pub total: usize,
}

/// The server the instance runs as, which the Export step offers to install beside the mods.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ServerSetup {
    /// The loader the server runs for.
    pub kind: ServerKind,
    /// The Minecraft version the server runs.
    pub minecraft: String,
    /// The loader's own version, when the loader has one.
    pub loader_version: Option<String>,
}

/// What the pack holds, once it is written.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Receipt {
    /// Where the pack was written.
    pub target: PathBuf,
    /// Mods the pack takes.
    pub taken: usize,
    /// Mods the pack leaves out.
    pub left_out: usize,
    /// Unsure mods the pack takes.
    pub unsure: usize,
    /// Mods with a choice the user made on the Review step.
    pub chosen: usize,
    /// Files written.
    pub files: usize,
}

/// The counts the Review step shows: the mods on each side, and what the pack takes.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub struct Counts {
    pub client: usize,
    pub server: usize,
    pub both: usize,
    pub unsure: usize,
    pub taken: usize,
    pub left_out: usize,
    pub chosen: usize,
}

/// What stops the export from starting, when something does.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Blocker {
    /// No destination is chosen yet.
    NoDestination,
    /// The pack's target already exists.
    Exists,
    /// The target lies inside the instance folder.
    Inside,
}

/// The wizard's state. The root starts its work and feeds it results; the steps show it.
#[derive(Debug, Clone, Default)]
pub struct State {
    /// The instance's id.
    pub instance: String,
    /// The instance's name, which names the pack.
    pub name: String,
    /// The instance's folder.
    pub folder: PathBuf,
    pub step: Step,
    pub tab: Tab,
    pub filter: Filter,
    pub search: String,
    pub sort: Sort,
    /// Every mod jar the scan read, in the order it read them.
    pub mods: Vec<ModRow>,
    /// The indices of the mods the list shows, in the order it shows them.
    pub visible: Vec<usize>,
    /// The choice for each mod, by [`pack::key`]. A mod with no entry is on Auto.
    pub choices: HashMap<String, Choice>,
    /// The indices of the rows the user selected.
    pub selected: HashSet<usize>,
    /// The top-level entries of the instance folder, except `mods`.
    pub folders: Vec<FolderRow>,
    /// The folders the pack takes.
    pub selection: Selection,
    /// The scan's latest jar.
    pub scan: ScanTick,
    /// How the scan's jars fell on each side.
    pub tally: Tally,
    pub kind: Kind,
    /// The server the instance runs as. `None` when the instance has no server to offer.
    pub setup: Option<ServerSetup>,
    /// Whether the pack installs the server beside the mods.
    pub install_server: bool,
    /// Whether the player accepted the Minecraft EULA for the installed server.
    pub accept_eula: bool,
    /// Whether the pack carries the Java runtime the server runs on, which its start scripts then use.
    pub include_java: bool,
    /// Whether the export installs a server before it writes the pack.
    pub installing: bool,
    /// The folder the pack is written into.
    pub destination: Option<PathBuf>,
    /// How far the export has got.
    pub export: ExportTick,
    /// The receipt, once the pack is written.
    pub receipt: Option<Receipt>,
    /// Set to stop the scan or the export at its next file.
    pub cancel: Arc<AtomicBool>,
}

/// The wizard's messages. The root handles the ones that start work or leave the page, and passes the rest to
/// [`State::update`].
#[derive(Debug, Clone)]
pub enum Message {
    /// Leaves the page, back to where it was opened from.
    Back,
    /// Stops the scan or the export that runs.
    Cancel,
    /// Switches the Review step's tab.
    Tab(Tab),
    /// Filters the mod list by side.
    Filter(Filter),
    /// The search text changed.
    Search(String),
    /// The mod list was sorted another way.
    Sort(Sort),
    /// A row was pressed: its selection toggles.
    Select(usize),
    /// The header checkbox was pressed.
    SelectVisible,
    /// The batch bar's clear button was pressed.
    ClearSelection,
    /// The Review step's rescan button was pressed. The root reads the mods again.
    Rescan,
    /// The mods at these indices take `choice`, where `None` puts them back on Auto. The root remembers it.
    Choose { mods: Vec<usize>, choice: Option<Choice> },
    /// The folder at this index in the Folders tab was ticked or unticked.
    Folder(usize),
    /// The Export step's output kind was picked.
    Kind(Kind),
    /// The Export step's server checkbox was ticked or unticked.
    InstallServer(bool),
    /// The Export step's EULA checkbox was ticked or unticked.
    AcceptEula(bool),
    /// The Export step's Java checkbox was ticked or unticked.
    IncludeJava(bool),
    /// The Export step's destination button was pressed.
    PickDestination,
    /// The folder picker answered with a folder, or was cancelled.
    DestinationPicked(Option<PathBuf>),
    /// The Review step continues to the Export step.
    Continue,
    /// The Export step goes back to the Review step.
    Review,
    /// The export starts.
    Export,
    /// The finished pack is shown in the file manager.
    Reveal,
    /// The scan read a jar.
    ScanTick(ScanTick),
    /// The scan finished, with its mods or the error it stopped at.
    Scanned(ScanDone),
    /// The choices the database remembers were read.
    ChoicesLoaded(Result<Vec<(String, String)>, String>),
    /// The top-level folders of the instance folder were listed, for the folder it names.
    FoldersListed(PathBuf, Result<Vec<FolderRow>, String>),
    /// The export wrote a file.
    ExportTick(ExportTick),
    /// The export finished, with the target it wrote and the files in it, or the error it stopped at.
    Exported(Result<(PathBuf, usize), String>),
    /// A remembered choice was written, or was not.
    ChoiceSaved(Result<(), String>),
}

impl State {
    /// A wizard for the instance `instance`, named `name`, in `folder`. It starts on the scan, with the folders a new
    /// pack leaves out already unticked.
    pub fn new(instance: String, name: String, folder: PathBuf, cancel: Arc<AtomicBool>) -> Self {
        let excluded = DEFAULT_EXCLUDED
            .iter()
            .map(|name| name.to_string())
            .chain(LEFT_OUT.iter().map(|name| name.to_string()));
        Self {
            instance,
            name,
            folder,
            cancel,
            selection: Selection::from_excluded(excluded),
            ..Self::default()
        }
    }

    /// Applies a message the root does not handle itself.
    pub fn update(&mut self, message: Message) {
        match message {
            Message::Tab(tab) => self.tab = tab,
            Message::Filter(filter) => {
                self.filter = filter;
                self.refresh();
            }
            Message::Search(search) => {
                self.search = search;
                self.refresh();
            }
            Message::Sort(sort) => {
                self.sort = sort;
                self.refresh();
            }
            Message::Select(index) => self.toggle_row(index),
            Message::SelectVisible => self.select_visible(),
            Message::ClearSelection => self.selected.clear(),
            Message::Folder(index) => self.toggle_folder(index),
            Message::Kind(kind) => self.kind = kind,
            Message::InstallServer(install) => self.install_server = install && self.server_available(),
            Message::AcceptEula(accepted) => self.accept_eula = accepted,
            Message::IncludeJava(include) => self.include_java = include,
            Message::Continue => self.step = Step::Export,
            Message::Review => self.step = Step::Review,
            _ => {}
        }
    }

    /// Takes the mods the scan read, and moves to the Review step.
    pub fn scanned(&mut self, mods: Vec<ModRow>) {
        self.mods = mods;
        self.selected.clear();
        self.step = Step::Review;
        self.refresh();
    }

    /// Whether a scan of `folder` is the one this page waits for: the wizard is scanning that instance folder.
    pub fn scanning_for(&self, folder: &Path) -> bool {
        self.step == Step::Scanning && self.folder == folder
    }

    /// Records a jar the scan just read, and tallies its side. A jar of another folder's scan is ignored.
    pub fn scan_tick(&mut self, tick: ScanTick) {
        if tick.folder != self.folder {
            return;
        }
        match tick.side.as_ref().map(side_rank) {
            Some(0) => self.tally.client += 1,
            Some(1) => self.tally.server += 1,
            Some(2) => self.tally.both += 1,
            _ => self.tally.unsure += 1,
        }
        self.scan = tick;
    }

    /// Takes the choices the database remembers. A choice already made in this session is kept.
    pub fn remember(&mut self, rows: Vec<(String, String)>) {
        for (mod_id, value) in rows {
            if let Some(choice) = stored_choice(&value) {
                self.choices.entry(mod_id).or_insert(choice);
            }
        }
    }

    /// Whether the instance's server can be installed beside the pack. Quilt's server is not installed by the launcher yet.
    pub fn server_available(&self) -> bool {
        self.setup.as_ref().is_some_and(|setup| setup.kind != ServerKind::Quilt)
    }

    /// Replaces the instance's folder list.
    pub fn folders_listed(&mut self, folders: Vec<FolderRow>) {
        self.folders = folders;
    }

    /// Sets the choice of the mods at `indices`, where `None` puts them back on Auto. Returns the choices the database
    /// should remember, as (mod id, choice) for each mod that has an id.
    pub fn choose(&mut self, indices: &[usize], choice: Option<Choice>) -> Vec<(String, Option<Choice>)> {
        let mut remember = Vec::new();
        for &index in indices {
            let Some(row) = self.mods.get(index) else {
                continue;
            };
            let key = pack::key(&row.detection);
            match choice {
                Some(choice) => {
                    self.choices.insert(key, choice);
                }
                None => {
                    self.choices.remove(&key);
                }
            }
            if let Some(mod_id) = row.detection.mod_id.clone() {
                remember.push((mod_id, choice));
            }
        }
        remember
    }

    /// The choice the mod at `index` has, if any.
    pub fn choice_of(&self, index: usize) -> Option<Choice> {
        let row = self.mods.get(index)?;
        self.choices.get(&pack::key(&row.detection)).copied()
    }

    /// Whether the pack takes the mod at `index`.
    pub fn takes(&self, index: usize) -> bool {
        self.mods.get(index).is_some_and(|row| pack::takes(&row.detection, self.choice_of(index)))
    }

    /// The jars the pack takes, from the instance's `mods` folder.
    pub fn planned_jars(&self) -> Vec<PathBuf> {
        (0..self.mods.len())
            .filter(|&index| self.takes(index))
            .map(|index| self.mods[index].detection.path.clone())
            .collect()
    }

    /// The counts the Review step shows.
    pub fn counts(&self) -> Counts {
        let mut counts = Counts::default();
        for (index, row) in self.mods.iter().enumerate() {
            match row.detection.side.as_ref().map(side_rank) {
                Some(0) => counts.client += 1,
                Some(1) => counts.server += 1,
                Some(2) => counts.both += 1,
                _ => counts.unsure += 1,
            }
            if self.takes(index) {
                counts.taken += 1;
            } else {
                counts.left_out += 1;
            }
            if self.choice_of(index).is_some() {
                counts.chosen += 1;
            }
        }
        counts
    }

    /// The name the pack is written under: the instance's name and "server pack", with `.zip` for a zip.
    pub fn pack_name(&self) -> String {
        let base = format!("{} server pack", self.name);
        match self.kind {
            Kind::Folder => file_safe(&base),
            Kind::Zip => file_safe(&format!("{base}.zip")),
        }
    }

    /// Where the pack is written, once a destination is chosen.
    pub fn target(&self) -> Option<PathBuf> {
        self.destination.as_ref().map(|destination| destination.join(self.pack_name()))
    }

    /// What stops the export from starting, when something does.
    pub fn blocker(&self) -> Option<Blocker> {
        let target = self.target()?;
        if target.starts_with(&self.folder) {
            return Some(Blocker::Inside);
        }
        if target.exists() {
            return Some(Blocker::Exists);
        }
        None
    }

    /// The destination's blocker, or `NoDestination` when none is chosen. The Export step's button waits for this.
    pub fn export_blocker(&self) -> Option<Blocker> {
        if self.destination.is_none() {
            Some(Blocker::NoDestination)
        } else {
            self.blocker()
        }
    }

    /// Takes the receipt of a finished export, which wrote `files` files to `target`.
    pub fn finish(&mut self, target: PathBuf, files: usize) {
        let counts = self.counts();
        let unsure = (0..self.mods.len())
            .filter(|&index| self.mods[index].detection.side.is_none() && self.takes(index))
            .count();
        self.receipt = Some(Receipt {
            target,
            taken: counts.taken,
            left_out: counts.left_out,
            unsure,
            chosen: counts.chosen,
            files,
        });
        self.step = Step::Done;
    }

    /// Records a finished export's progress.
    pub fn export_tick(&mut self, tick: ExportTick) {
        self.export = tick;
    }

    /// Moves to the Exporting step, with no file written yet. `installing` says whether a server is installed first.
    pub fn begin_export(&mut self, installing: bool) {
        self.export = ExportTick::default();
        self.installing = installing;
        self.step = Step::Exporting;
    }

    /// Moves back to the Export step after the export stopped without a pack.
    pub fn export_failed(&mut self) {
        self.step = Step::Export;
    }

    /// Recomputes the mods the list shows, from the filter, the search and the sort.
    pub fn refresh(&mut self) {
        let query = self.search.trim().to_lowercase();
        let mut visible: Vec<usize> = (0..self.mods.len())
            .filter(|&index| {
                let detection = &self.mods[index].detection;
                matches_filter(self.filter, &detection.side) && (query.is_empty() || detection.filename.to_lowercase().contains(&query))
            })
            .collect();
        visible.sort_by(|&a, &b| compare(self.sort, &self.mods[a].detection, &self.mods[b].detection));
        self.visible = visible;
    }

    /// Toggles the selection of the row at `index`.
    fn toggle_row(&mut self, index: usize) {
        if !self.selected.remove(&index) {
            self.selected.insert(index);
        }
    }

    /// Selects every visible row, or clears them all when they are all selected already.
    fn select_visible(&mut self) {
        let all = !self.visible.is_empty() && self.visible.iter().all(|index| self.selected.contains(index));
        for index in self.visible.clone() {
            if all {
                self.selected.remove(&index);
            } else {
                self.selected.insert(index);
            }
        }
    }

    /// Ticks or unticks the folder at `index`.
    fn toggle_folder(&mut self, index: usize) {
        let Some(folder) = self.folders.get(index) else {
            return;
        };
        let name = folder.name.clone();
        let included = self.selection.is_included(&name);
        self.selection.set(&name, !included, |_| Vec::new());
    }

    /// Whether the folder at `index` is ticked.
    pub fn folder_ticked(&self, index: usize) -> bool {
        self.folders.get(index).is_some_and(|folder| self.selection.is_included(&folder.name))
    }

    /// The wizard's current step, drawn in full.
    pub fn view(&self) -> Element<'_, Message> {
        match self.step {
            Step::Scanning => progress::scanning(self),
            Step::Review => review::view(self),
            Step::Export => export::view(self),
            Step::Exporting => progress::exporting(self),
            Step::Done => done::view(self),
        }
    }
}

/// The side rank of a detected side, as the mod list sorts it: client, server, both, then unsure.
fn side_rank(side: &Side) -> u8 {
    match side {
        Side::ClientOnly => 0,
        Side::ServerOnly => 1,
        Side::ClientAndServer => 2,
    }
}

/// Whether a mod with `side` shows under `filter`.
fn matches_filter(filter: Filter, side: &Option<Side>) -> bool {
    match filter {
        Filter::All => true,
        Filter::Client => *side == Some(Side::ClientOnly),
        Filter::Server => *side == Some(Side::ServerOnly),
        Filter::Both => *side == Some(Side::ClientAndServer),
        Filter::Unsure => side.is_none(),
    }
}

/// The order two mods show in under `sort`.
fn compare(sort: Sort, a: &ModDetection, b: &ModDetection) -> Ordering {
    let by_name = a.filename.to_lowercase().cmp(&b.filename.to_lowercase());
    match sort {
        Sort::Name => by_name,
        Sort::Side => {
            let rank = |side: &Option<Side>| side.as_ref().map_or(3, side_rank);
            rank(&a.side).cmp(&rank(&b.side)).then(by_name)
        }
    }
}

/// The text a choice is stored as in the database.
pub fn stored(choice: Choice) -> &'static str {
    match choice {
        Choice::Include => "include",
        Choice::Exclude => "exclude",
    }
}

/// The choice a stored value names, when it names one.
pub fn stored_choice(value: &str) -> Option<Choice> {
    match value {
        "include" => Some(Choice::Include),
        "exclude" => Some(Choice::Exclude),
        _ => None,
    }
}

#[cfg(test)]
#[path = "../../../../tests/app/pages/server_pack/mod.rs"]
mod tests;
