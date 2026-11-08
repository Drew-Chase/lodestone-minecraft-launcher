//! The Worlds page: every saved world across every instance, as a grid of cards or a table, with a search, filter
//! chips and a sort. The Backups chip lists every instance's backups as rows in place of the worlds. Each time the
//! page opens, the saves and backups are read from every instance's `saves/` and `backups/` folders on the blocking
//! pool. The actions a world offers are the root's (`app/world_actions.rs`), and each names the instance its world
//! belongs to.

mod filters;
pub(crate) mod format;
mod header;
mod list;
mod scan;

use std::collections::HashSet;
use std::path::{Path, PathBuf};

use chrono::{DateTime, Local};
use iced::widget::{Column, container};
use iced::{Element, Fill, Padding, Size};

pub use scan::{BackupEntry, Scan, Source, WorldEntry};

use crate::app::components::scrollbar;
use crate::app::lang;
use crate::app::pages::instance::files::WorldFile;
use crate::app::pages::instance::format::format_bytes;
use crate::app::world_actions::WorldMessage;
use crate::app::world_pins::WorldPins;
use filters::{WorldFilter, WorldSort, WorldView};

/// A menu the page opens beside its trigger: the sort orders, or the instances a new world or an import goes into.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Menu {
    Sort,
    New,
    Import,
}

/// Messages the Worlds page emits and handles.
#[derive(Debug, Clone)]
pub enum Message {
    Search(String),
    Filter(WorldFilter),
    Sort(WorldSort),
    View(WorldView),
    Scrolled(f32),
    /// The pointer entered (`hovered`) or left the card of the save at this path.
    Hover {
        path: PathBuf,
        hovered: bool,
    },
    /// A menu's trigger was pressed: the menu opens, or closes when it is open.
    ToggleMenu(Menu),
    /// A press landed outside the open menu, or Escape was pressed.
    DismissMenu,
    /// An instance was picked in a menu. The root starts the menu's action on it.
    Pick {
        menu: Menu,
        instance: String,
    },
    /// A world's request, which the root carries out.
    World(WorldMessage),
    /// The saves and the backups were read. Stale when `generation` has moved on.
    Scanned {
        generation: u64,
        scan: Scan,
    },
}

/// The figures the header's summary shows.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
struct Totals {
    worlds: usize,
    instances: usize,
    bytes: u64,
}

impl Totals {
    fn of(entries: &[WorldEntry]) -> Self {
        let instances: HashSet<&str> = entries.iter().map(|entry| entry.instance_id.as_str()).collect();
        Self {
            worlds: entries.len(),
            instances: instances.len(),
            bytes: entries.iter().map(|entry| entry.world.size_bytes).sum(),
        }
    }
}

/// The Worlds page's state.
#[derive(Debug, Default)]
pub struct State {
    /// Every save of every instance, as the last scan read them.
    entries: Vec<WorldEntry>,
    /// Every backup of every instance, newest first, as the last scan read them.
    backups: Vec<BackupEntry>,
    /// Whether a scan has landed. Until then the page says the saves are being read.
    loaded: bool,
    /// Bumped by every scan. A result from an older scan is dropped.
    generation: u64,
    /// When the last scan began: the reference for the "last played" times.
    now: Option<DateTime<Local>>,
    /// The instances the last scan read, by id and name, for the New world and Import menus.
    instances: Vec<(String, String)>,
    /// The pinned saves, as the root holds them.
    pins: WorldPins,
    query: String,
    filter: WorldFilter,
    sort: WorldSort,
    view: WorldView,
    scroll_y: f32,
    /// The save whose card the pointer is over, if any.
    hovered: Option<PathBuf>,
    /// The menu that is open, if any.
    menu: Option<Menu>,
    /// Indices into `entries` that pass the search and the filter, in the sort order. `refilter` keeps it current.
    visible: Vec<usize>,
    /// Indices into `backups` that pass the search. `refilter` keeps it current.
    visible_backups: Vec<usize>,
    /// How many each filter chip lists, in `WorldFilter::ALL` order: worlds, or backups for the Backups chip.
    counts: [usize; WorldFilter::ALL.len()],
    totals: Totals,
}

impl State {
    /// Starts a scan of `sources`. A result from any earlier scan is dropped when it lands. Returns the generation
    /// the scan reports.
    pub fn begin_scan(&mut self, sources: &[Source]) -> u64 {
        self.generation += 1;
        self.now = Some(Local::now());
        self.instances = sources.iter().map(|source| (source.id.clone(), source.name.clone())).collect();
        self.generation
    }

    /// Takes the pins the root holds, and lists the worlds again so the pinned filter and the badges are current.
    pub fn set_pins(&mut self, pins: WorldPins) {
        self.pins = pins;
        self.refilter();
    }

    /// The save at `path` of the instance `instance`, when the page lists it.
    pub fn world(&self, instance: &str, path: &Path) -> Option<&WorldFile> {
        self.entries
            .iter()
            .find(|entry| entry.instance_id == instance && entry.world.path.as_path() == path)
            .map(|entry| &entry.world)
    }

    /// The save in the folder `folder` of the instance `instance`, when the page lists it. A backup names its save
    /// by folder, so this is how its row finds the save's art.
    fn saved_world(&self, instance: &str, folder: &str) -> Option<&WorldFile> {
        self.entries
            .iter()
            .find(|entry| entry.instance_id == instance && entry.world.path.file_name().and_then(|name| name.to_str()) == Some(folder))
            .map(|entry| &entry.world)
    }

    /// Drops the save at `path` of the instance `instance`, as it went to the trash.
    pub fn remove(&mut self, instance: &str, path: &Path) {
        self.entries
            .retain(|entry| entry.instance_id != instance || entry.world.path.as_path() != path);
        self.refilter();
    }

    pub fn update(&mut self, message: Message) {
        match message {
            Message::Search(query) => {
                self.query = query;
                self.refilter();
            }
            Message::Filter(filter) => {
                self.filter = filter;
                self.menu = None;
                self.refilter();
            }
            Message::Sort(sort) => {
                self.sort = sort;
                self.menu = None;
                self.refilter();
            }
            Message::View(view) => self.view = view,
            Message::Scrolled(offset) => self.scroll_y = offset.max(0.0),
            // An exit clears only the card it was for, so enter and exit can arrive in any order.
            Message::Hover { path, hovered } => {
                if hovered {
                    self.hovered = Some(path);
                } else if self.hovered.as_deref() == Some(path.as_path()) {
                    self.hovered = None;
                }
            }
            Message::ToggleMenu(menu) => self.menu = if self.menu == Some(menu) { None } else { Some(menu) },
            Message::DismissMenu | Message::Pick { .. } => self.menu = None,
            Message::Scanned { generation, scan } => {
                if generation == self.generation {
                    self.entries = scan.worlds;
                    self.backups = scan.backups;
                    self.loaded = true;
                    self.refilter();
                }
            }
            // The root carries these out.
            Message::World(_) => {}
        }
    }

    /// Recomputes what the list shows, the chip counts and the totals.
    fn refilter(&mut self) {
        let query = self.query.to_lowercase();
        let (pins, entries) = (&self.pins, &self.entries);
        let pinned: Vec<bool> = entries
            .iter()
            .map(|entry| pins.contains(&entry.instance_id, &entry.world.folder()))
            .collect();

        let mut counts = [0; WorldFilter::ALL.len()];
        for (count, filter) in counts.iter_mut().zip(WorldFilter::ALL) {
            *count = match filter {
                WorldFilter::Backups => self.backups.len(),
                _ => (0..entries.len()).filter(|&index| filter.matches(&entries[index], pinned[index])).count(),
            };
        }
        let mut visible: Vec<usize> = (0..entries.len())
            .filter(|&index| self.filter.matches(&entries[index], pinned[index]) && entries[index].matches(&query))
            .collect();
        let sort = self.sort;
        if sort == WorldSort::Name {
            // The name is lowered once per world, not once per comparison. The index breaks ties, so equal names keep their order.
            visible.sort_by_cached_key(|&index| (entries[index].world.name.to_lowercase(), index));
        } else {
            visible.sort_by(|&a, &b| sort.compare(&entries[a], &entries[b]));
        }
        let visible_backups = (0..self.backups.len()).filter(|&index| self.backups[index].matches(&query)).collect();

        self.counts = counts;
        self.totals = Totals::of(entries);
        self.visible = visible;
        self.visible_backups = visible_backups;
    }

    /// The header's line: how many worlds, across how many instances, and their size. Until a scan lands it says the
    /// saves are being read.
    fn summary(&self) -> String {
        if !self.loaded {
            return lang::t("worlds.loading").to_owned();
        }
        let worlds = lang::t_count("worlds.count", self.totals.worlds);
        let instances = lang::t_count("worlds.instances", self.totals.instances);
        let size = format_bytes(self.totals.bytes);
        lang::t_with(
            "worlds.summary",
            &[("worlds", worlds.as_str()), ("instances", instances.as_str()), ("size", size.as_str())],
        )
    }

    /// The page: the header stays in place, and the toolbar and the list scroll beneath it.
    pub fn view(&self, viewport: Size) -> Element<'_, Message> {
        let fixed = container(header::view(self))
            .padding(Padding::new(0.0).top(24.0).left(28.0).right(28.0))
            .width(Fill);
        let content = Column::new()
            .push(filters::view(self))
            .push(list::view(self, viewport))
            .spacing(16)
            .padding(Padding::new(0.0).left(28.0).right(28.0).bottom(28.0))
            .width(Fill);
        let scrolled = scrollbar::scroll(content)
            .width(Fill)
            .height(Fill)
            .on_scroll(|scroll| Message::Scrolled(scroll.absolute_offset().y));
        Column::new().push(fixed).push(scrolled).width(Fill).height(Fill).into()
    }
}

#[cfg(test)]
#[path = "../../../../tests/app/pages/worlds/mod.rs"]
mod tests;
