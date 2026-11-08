//! The Library — the launcher's landing page: the most-recent instance
//! hero, the quick actions, and the instance list (Grid / Compact / Table
//! views, filter tabs, search). The header stays in place, and everything
//! under it scrolls as one — the instance list is virtualized against that
//! scroll, so nothing is trapped in its own viewport on short windows.

mod hero;
pub mod instance;
pub mod instance_menu;
mod list;
pub mod new_instance;
mod toolbar;

use instance::{Instance, Loader};
use new_instance::NewInstance;

use crate::app::components::art_cache::ArtCache;
use crate::app::components::instance_card::{CARD_ART_HEIGHT, CARD_HEIGHT, COMPACT_ROW_HEIGHT, TABLE_ROW_HEIGHT};
use crate::app::components::scrollbar;
use crate::app::downloads::Pending;
use crate::app::install::InstallRequest;
use crate::app::lang;
use crate::app::notifications;
use crate::app::paths::LauncherPaths;
use crate::app::theme::fonts;
use crate::app::updater::Button;
use iced::widget::{Column, Text, container, svg};
use iced::{Background, Border, Color, Element, Fill, Padding, Size, Task};
use tracing::info;

/// Messages the Library page handles.
#[derive(Debug, Clone)]
pub enum Message {
    /// The search query changed — filters the instance list by name.
    SearchChanged(String),
    /// A filter tab was selected.
    FilterChanged(LibraryFilter),
    /// The view mode changed (Grid / Compact / Table).
    ViewChanged(LibraryView),
    /// The page scrolled — drives the virtualized windowing.
    Scrolled(f32),
    /// Play was pressed on the instance with this id. The root launches it, or opens its console while it runs.
    Play(String),
    /// Another process of the playing instance with this id was asked for from the hero. The root starts it.
    LaunchNew(String),
    /// Open an instance's detail page — the root routes to it.
    OpenInstance(String),
    /// Edit an instance: open its detail page on the Settings tab — the root routes to it.
    EditInstance(String),
    /// Reveal an instance's folder in the file manager — the root does the IO.
    RevealInstance(String),
    /// A row of an instance's menu was picked. A separator sends `None`.
    InstanceMenu {
        instance: String,
        action: Option<instance_menu::Action>,
    },
    /// The pointer entered (`hovered`) or left a card, which grows while it is hovered.
    Hover { instance: String, hovered: bool },
    /// The Join Server card opens the Join Server dialog. The root opens it.
    OpenJoinServer,
    /// The Co-op Sync card opens the Co-op Sync dialog. The root opens it.
    OpenCoopSync,
    /// The update button in the header was pressed. The root starts the update, or opens its page.
    UpdatePressed,
    /// The refresh button — the root re-queries the database.
    Reload,
    /// The New Instance button opens the dialog. The root loads its lists.
    OpenNewInstance,
    /// The Import card opens the Import Instance dialog. The root reads the recent imports it lists.
    OpenImport,
    /// A message for the New Instance dialog.
    NewInstance(new_instance::Message),
    /// The bell in the header and its panel. The root applies them, since the notifications are kept there.
    Notifications(notifications::Message),
}

/// The Library's view modes.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum LibraryView {
    /// Full cards with scene art — the reference layout.
    #[default]
    Grid,
    /// The grid card's info without the art, one tight row.
    Compact,
    /// A traditional table.
    Table,
}

/// The least width of a compact row before the list wraps to another column. Two rows fit a 1280px window,
/// and each one has room for its title, stats, chip and Play button.
const COMPACT_MIN_WIDTH: f32 = 440.0;

impl LibraryView {
    /// The virtualized row pitch (px): one item's height plus the gap.
    fn row_pitch(self) -> f32 {
        Self::item_height(self) + Self::gap(self)
    }

    /// One item's fixed height in this view.
    fn item_height(self) -> f32 {
        match self {
            LibraryView::Grid => CARD_HEIGHT,
            LibraryView::Compact => COMPACT_ROW_HEIGHT,
            LibraryView::Table => TABLE_ROW_HEIGHT,
        }
    }

    /// The gap between rows in this view.
    fn gap(self) -> f32 {
        match self {
            LibraryView::Grid | LibraryView::Compact => 24.0,
            LibraryView::Table => 0.0,
        }
    }

    /// How many columns fit in `width`: the grid tiles at a 340px minimum card width, the compact rows at
    /// [`COMPACT_MIN_WIDTH`], both capped at 4 so wide windows keep the reference's large cards. The table
    /// is one column.
    fn columns(self, width: f32) -> usize {
        match self {
            LibraryView::Grid => ((width / 340.0) as usize).clamp(1, 4),
            LibraryView::Compact => (((width + self.gap()) / (COMPACT_MIN_WIDTH + self.gap())) as usize).clamp(1, 4),
            LibraryView::Table => 1,
        }
    }

    fn label(self) -> &'static str {
        match self {
            LibraryView::Grid => lang::t("library.view.grid"),
            LibraryView::Compact => lang::t("library.view.compact"),
            LibraryView::Table => lang::t("library.view.table"),
        }
    }
}

/// The instance-list filter tabs.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum LibraryFilter {
    /// Every instance, most recent first.
    #[default]
    Recent,
    /// Only pinned instances.
    Pinned,
    /// Modded loaders only.
    Modded,
    /// Vanilla only.
    Vanilla,
}

impl LibraryFilter {
    fn label(self) -> &'static str {
        match self {
            LibraryFilter::Recent => lang::t("library.filter.recent"),
            LibraryFilter::Pinned => lang::t("library.filter.pinned"),
            LibraryFilter::Modded => lang::t("library.filter.modded"),
            LibraryFilter::Vanilla => lang::t("library.filter.vanilla"),
        }
    }

    fn matches(self, instance: &Instance) -> bool {
        match self {
            LibraryFilter::Recent => true,
            LibraryFilter::Pinned => instance.pinned,
            LibraryFilter::Modded => instance.loader != Loader::Vanilla,
            LibraryFilter::Vanilla => instance.loader == Loader::Vanilla,
        }
    }
}

/// The page header with its top padding. It stays above the scrolling content, and the content's own top
/// padding is the gap below it. Like `LIST_TOP`, it is an estimate.
const HEADER_BLOCK: f32 = 73.0;

/// Vertical distance from the top of the scrolling content down to the first list row: top padding +
/// hero + quick actions + section title, plus the spacings between them. The header is above the scroll,
/// so it is not counted. The virtualized window only needs this within one row pitch — the overscan
/// absorbs the drift — so it stays a constant instead of a measurement.
const LIST_TOP: f32 = 497.0;

/// The grid card's art box when `columns` cards share a `content` width: the card's width less its
/// border, and the art's height less the pixel it sits down by.
pub(super) fn card_art_size(content: f32, columns: usize) -> (f32, f32) {
    let card = (content - LibraryView::Grid.gap() * (columns as f32 - 1.0)) / columns as f32;
    (card - 2.0, CARD_ART_HEIGHT - 1.0)
}

/// The Library page state.
#[derive(Debug, Default)]
pub struct Library {
    /// Every instance, loaded from the database.
    instances: Vec<Instance>,
    /// The search query — filters the list by name.
    search: String,
    /// The selected filter tab.
    filter: LibraryFilter,
    /// The selected view mode.
    view: LibraryView,
    /// The page scroll offset — drives the virtualized windowing.
    scroll_y: f32,
    /// The New Instance dialog, while it is open.
    new_instance: Option<NewInstance>,
    /// The instance whose card the pointer is over, if any.
    hovered: Option<String>,
    /// The installs still running or waiting, shown as cards beside the instances.
    pending: Vec<Pending>,
    /// The positions, in `every_instance`, of the instances the list shows, in order. `refilter` keeps it current, so the
    /// list is not searched and sorted on every frame.
    visible: Vec<usize>,
}

impl Library {
    /// Applies a display-state change. Database effects (`Reload`) are
    /// intercepted by the root before reaching here. The scroll offset is
    /// owned by the page scroll alone — filter changes must not jump it.
    pub fn update(&mut self, message: Message) {
        match message {
            Message::SearchChanged(search) => {
                self.search = search;
                self.refilter();
            }
            Message::FilterChanged(filter) => {
                self.filter = filter;
                self.refilter();
            }
            Message::ViewChanged(view) => self.view = view,
            Message::Scrolled(offset) => self.scroll_y = offset.max(0.0),
            Message::Hover { instance, hovered } => {
                // An exit clears only the card it was for, so enter and exit can arrive in any order.
                if hovered {
                    self.hovered = Some(instance);
                } else if self.hovered.as_deref() == Some(instance.as_str()) {
                    self.hovered = None;
                }
            }
            // The root launches the game: it needs the launch state and the account.
            Message::Play(_) | Message::LaunchNew(_) => {}
            Message::OpenInstance(id) => info!("Open instance {id} requested"),
            // The root routes these: they need the database, the alerts or the instance page.
            Message::EditInstance(_) | Message::RevealInstance(_) | Message::InstanceMenu { .. } => {}
            // The root handles the reload, the update and the New Instance messages: they need the database or the updater.
            Message::Reload
            | Message::OpenNewInstance
            | Message::OpenImport
            | Message::OpenJoinServer
            | Message::OpenCoopSync
            | Message::NewInstance(_)
            | Message::UpdatePressed
            | Message::Notifications(_) => {}
        }
    }

    /// Replaces the loaded instances (the root re-queried the database).
    pub fn set_instances(&mut self, instances: Vec<Instance>) {
        self.instances = instances;
        self.refilter();
    }

    /// Looks an instance up by id — the root uses this to open the detail
    /// page when a card or the hero is clicked. An install still running counts.
    /// The instances the library holds, in the order it lists them.
    pub fn instances(&self) -> &[Instance] {
        &self.instances
    }

    pub fn instance(&self, id: &str) -> Option<Instance> {
        self.every_instance().find(|instance| instance.id == id).cloned()
    }

    /// The folder names of every instance, including the installs still running. A new install must avoid them.
    pub fn directory_names(&self) -> Vec<String> {
        self.every_instance().map(|instance| instance.directory_name.clone()).collect()
    }

    /// The instances that came from a content platform, as (platform key, project id, instance id).
    pub fn installed_projects(&self) -> Vec<(String, String, String)> {
        self.instances
            .iter()
            .filter_map(|instance| {
                Some((
                    instance.source_platform.clone()?,
                    instance.source_project_id.clone()?,
                    instance.id.clone(),
                ))
            })
            .collect()
    }

    /// The name of the instance `id`, when the library lists it.
    pub fn instance_name(&self, id: &str) -> Option<&str> {
        self.every_instance()
            .find(|instance| instance.id == id)
            .map(|instance| instance.name.as_str())
    }

    /// The loaded instances, then the installs still running.
    fn every_instance(&self) -> impl Iterator<Item = &Instance> {
        self.instances.iter().chain(self.pending.iter().map(|pending| &pending.instance))
    }

    /// Replaces the installs that have not ended. The root sends them whenever the queue changes.
    pub fn set_pending(&mut self, pending: Vec<Pending>) {
        self.pending = pending;
        self.refilter();
    }

    /// The install of the instance `id`, while it has not ended.
    pub fn pending_install(&self, id: &str) -> Option<&Pending> {
        self.pending.iter().find(|pending| pending.instance.id == id)
    }

    /// Adds a freshly installed instance, or replaces the copy the library holds of it.
    pub fn add_instance(&mut self, instance: Instance) {
        match self.instances.iter_mut().find(|known| known.id == instance.id) {
            Some(known) => *known = instance,
            None => self.instances.push(instance),
        }
        self.refilter();
    }

    /// Overrides the initial view mode (the `LODESTONE_LIBRARY_VIEW` probe).
    pub fn set_view(&mut self, view: LibraryView) {
        self.view = view;
    }

    /// The instances after the search query and filter tab, most recent
    /// first.
    fn visible(&self) -> Vec<&Instance> {
        let all: Vec<&Instance> = self.every_instance().collect();
        self.visible.iter().filter_map(|&index| all.get(index).copied()).collect()
    }

    /// Recomputes which instances the list shows, and their order, from the search and the filter. The loaded instances and the
    /// installs still running are searched together, and the most recently played comes first.
    fn refilter(&mut self) {
        let query = self.search.to_lowercase();
        let all: Vec<&Instance> = self.every_instance().collect();
        let mut visible: Vec<usize> = (0..all.len())
            .filter(|&index| self.filter.matches(all[index]))
            .filter(|&index| query.is_empty() || all[index].name.to_lowercase().contains(&query))
            .collect();
        visible.sort_by_key(|&index| std::cmp::Reverse(all[index].recency()));
        self.visible = visible;
    }

    /// Opens the New Instance dialog at `paths`. The task loads the lists it offers.
    pub fn open_new_instance(
        &mut self,
        paths: LauncherPaths,
        cached: Option<std::sync::Arc<crate::app::install::catalog::Releases>>,
    ) -> Task<Message> {
        let dialog = NewInstance::new(paths);
        let load = dialog.load(cached).map(Message::NewInstance);
        self.new_instance = Some(dialog);
        load
    }

    /// Applies a dialog message that needs no database. Closing drops the dialog.
    pub fn new_instance_update(&mut self, message: new_instance::Message) -> Task<Message> {
        if let new_instance::Message::Close = message {
            self.new_instance = None;
            return Task::none();
        }
        match self.new_instance.as_mut() {
            Some(dialog) => dialog.update(message).map(Message::NewInstance),
            None => Task::none(),
        }
    }

    /// The install the dialog describes, avoiding the folder names in `taken`. A form that cannot install
    /// says why. Submit only comes from an open dialog, so the empty error is never shown.
    pub fn new_instance_request(&self, taken: &[String]) -> Result<InstallRequest, String> {
        self.new_instance
            .as_ref()
            .map_or_else(|| Err(String::new()), |dialog| dialog.request(taken))
    }

    /// Records why the form could not install. The dialog stays open to try again.
    pub fn fail_new_instance(&mut self, error: String) {
        if let Some(dialog) = self.new_instance.as_mut() {
            dialog.show_error(error);
        }
    }

    /// Closes the dialog once its install is queued.
    pub fn close_new_instance(&mut self) {
        self.new_instance = None;
    }

    /// The New Instance dialog, when it is open, for the root to stack over the page.
    pub fn modal(&self) -> Option<Element<'_, Message>> {
        self.new_instance.as_ref().map(|dialog| dialog.view().map(Message::NewInstance))
    }

    /// The page: the header stays where it is, and the hero, the quick actions and the instance list
    /// scroll beneath it. `viewport` is the window's inner size, used for the grid's column count and the
    /// windowed row count.
    pub fn view<'a>(
        &'a self,
        viewport: Size,
        art: &'a ArtCache,
        update: Option<Button>,
        notifications: &'a notifications::Notifications,
    ) -> Element<'a, Message> {
        let fixed = container(self.header(update, notifications))
            .padding(Padding::new(0.0).top(24).left(28).right(28))
            .width(Fill);
        // The banner is optional, so the column takes its children one at a
        // time: a missing hero must not leave a spacing gap behind.
        let mut page = Column::new();
        if let Some(hero) = self.hero(viewport, art) {
            page = page.push(hero);
        }
        let page = page.push(self.quick_actions()).push(self.instances_section(viewport, art));

        let scrolled = scrollbar::scroll(
            page.spacing(24)
                .padding(Padding::new(0.0).left(28).right(28).top(24).bottom(28))
                .width(Fill),
        )
        .width(Fill)
        .height(Fill)
        .on_scroll(|scroll| Message::Scrolled(scroll.absolute_offset().y));
        Column::new().push(fixed).push(scrolled).width(Fill).height(Fill).into()
    }
}

/// An uppercase mono chip on a tinted fill.
fn chip(label: String, ink: Color, fill: Color) -> Element<'static, Message> {
    container(Text::new(label).size(10).font(fonts::JetBrainsMono::SEMI_BOLD).color(ink))
        .padding(Padding::new(0.0).left(8).right(8).top(3).bottom(3))
        .style(move |_| container::Style {
            background: Some(Background::Color(fill)),
            border: Border {
                radius: 999.0.into(),
                ..Border::default()
            },
            ..container::Style::default()
        })
        .into()
}

/// An SVG icon from a lucide markup string. The tint supplies the color
/// (its alpha is ignored by the svg texture pipeline); `opacity` controls
/// visibility without shifting layout.
fn icon(markup: &'static str, size: f32, tint: Color, opacity: f32) -> Element<'static, Message> {
    svg(iced::widget::svg::Handle::from_memory(markup.as_bytes()))
        .width(size)
        .height(size)
        .opacity(opacity)
        .style(move |_, _| svg::Style { color: Some(tint) })
        .into()
}

#[cfg(test)]
#[path = "../../../../tests/app/pages/library/mod.rs"]
mod tests;
