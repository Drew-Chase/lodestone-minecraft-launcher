//! The Library — the launcher's landing page: the most-recent instance
//! hero, the quick actions, and the instance list (Grid / Compact / Table
//! views, filter tabs, search). The WHOLE page scrolls as one — the
//! instance list is virtualized against that page scroll, so nothing is
//! trapped in its own viewport on short windows.

mod hero;
pub mod instance;
pub mod instance_menu;
mod list;
pub mod new_instance;
mod toolbar;

use instance::{Instance, Loader};
use new_instance::NewInstance;

use crate::app::components::instance_card::{CARD_HEIGHT, COMPACT_ROW_HEIGHT, TABLE_ROW_HEIGHT};
use crate::app::components::scrollbar;
use crate::app::database::Database;
use crate::app::lang;
use crate::app::paths::LauncherPaths;
use crate::app::theme::fonts;
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
    /// A row of an instance's menu was picked; `instance_menu::action_at` says what it asks for.
    InstanceMenu { instance: String, index: usize },
    /// The pointer entered (`hovered`) or left a card, which grows while it is hovered.
    Hover { instance: String, hovered: bool },
    /// A header/quick action (Import, Join Server, ...) — dummy.
    QuickAction(&'static str),
    /// The refresh button — the root re-queries the database.
    Reload,
    /// The New Instance button opens the dialog. The root loads its lists.
    OpenNewInstance,
    /// A message for the New Instance dialog.
    NewInstance(new_instance::Message),
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

    /// How many columns fit in `width` — the grid tiles at a 340px minimum
    /// card width, capped at 4 so wide windows keep the reference's large
    /// cards; the other views are single-column lists.
    fn columns(self, width: f32) -> usize {
        match self {
            LibraryView::Grid => ((width / 340.0) as usize).clamp(1, 4),
            LibraryView::Compact | LibraryView::Table => 1,
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

/// Vertical distance from the top of the page content down to the first
/// list row: top padding + header + hero + quick actions + section title,
/// plus the spacings between them. The virtualized window only needs this
/// within one row pitch — the overscan absorbs the drift — so it stays a
/// constant instead of a measurement.
const LIST_TOP: f32 = 570.0;

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
}

impl Library {
    /// Applies a display-state change. Database effects (`Reload`) are
    /// intercepted by the root before reaching here. The scroll offset is
    /// owned by the page scroll alone — filter changes must not jump it.
    pub fn update(&mut self, message: Message) {
        // Any message but a hover ends the hover: the card under the pointer may have moved or
        // gone, and then its exit would never arrive.
        if !matches!(message, Message::Hover { .. }) {
            self.hovered = None;
        }
        match message {
            Message::SearchChanged(search) => self.search = search,
            Message::FilterChanged(filter) => self.filter = filter,
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
            Message::QuickAction(action) => info!("Quick action '{action}' (dummy)"),
            // The root handles the reload and the New Instance messages: they need the database.
            Message::Reload | Message::OpenNewInstance | Message::NewInstance(_) => {}
        }
    }

    /// Replaces the loaded instances (the root re-queried the database).
    pub fn set_instances(&mut self, instances: Vec<Instance>) {
        self.instances = instances;
    }

    /// Looks an instance up by id — the root uses this to open the detail
    /// page when a card or the hero is clicked.
    pub fn instance(&self, id: &str) -> Option<Instance> {
        self.instances.iter().find(|instance| instance.id == id).cloned()
    }

    /// The name of the instance `id`, when the library lists it.
    pub fn instance_name(&self, id: &str) -> Option<&str> {
        self.instances
            .iter()
            .find(|instance| instance.id == id)
            .map(|instance| instance.name.as_str())
    }

    /// Overrides the initial view mode (the `LODESTONE_LIBRARY_VIEW` probe).
    pub fn set_view(&mut self, view: LibraryView) {
        self.view = view;
    }

    /// The instances after the search query and filter tab, most recent
    /// first.
    fn visible(&self) -> Vec<&Instance> {
        let query = self.search.to_lowercase();
        let mut visible: Vec<&Instance> = self
            .instances
            .iter()
            .filter(|instance| self.filter.matches(instance))
            .filter(|instance| query.is_empty() || instance.name.to_lowercase().contains(&query))
            .collect();
        visible.sort_by_key(|instance| std::cmp::Reverse(instance.recency()));
        visible
    }

    /// Opens the New Instance dialog at `paths`. The task loads the lists it offers.
    pub fn open_new_instance(&mut self, paths: LauncherPaths) -> Task<Message> {
        let dialog = NewInstance::new(paths);
        let load = dialog.load().map(Message::NewInstance);
        self.new_instance = Some(dialog);
        load
    }

    /// Applies a dialog message that needs no database. Closing keeps an install running.
    pub fn new_instance_update(&mut self, message: new_instance::Message) -> Task<Message> {
        if let new_instance::Message::Close = message {
            if self.new_instance.as_ref().is_some_and(|dialog| !dialog.is_installing()) {
                self.new_instance = None;
            }
            return Task::none();
        }
        match self.new_instance.as_mut() {
            Some(dialog) => dialog.update(message).map(Message::NewInstance),
            None => Task::none(),
        }
    }

    /// Starts the install the dialog describes. A form that cannot install says why.
    pub fn start_new_instance(&mut self, db: Database) -> Task<Message> {
        let Some(dialog) = self.new_instance.as_mut() else {
            return Task::none();
        };
        match dialog.start_install(db) {
            Ok(task) => task.map(Message::NewInstance),
            Err(error) => {
                dialog.install_failed(error);
                Task::none()
            }
        }
    }

    /// Stops the running install. Its folder is removed as the install is dropped.
    pub fn cancel_new_instance(&mut self) {
        if let Some(dialog) = self.new_instance.as_mut() {
            dialog.cancel_install();
        }
    }

    /// Records why an install stopped. The dialog stays open to try again.
    pub fn fail_new_instance(&mut self, error: String) {
        if let Some(dialog) = self.new_instance.as_mut() {
            dialog.install_failed(error);
        }
    }

    /// Closes the dialog once its instance exists.
    pub fn close_new_instance(&mut self) {
        self.new_instance = None;
    }

    /// The New Instance dialog, when it is open, for the root to stack over the page.
    pub fn modal(&self) -> Option<Element<'_, Message>> {
        self.new_instance.as_ref().map(|dialog| dialog.view().map(Message::NewInstance))
    }

    /// The whole page as one scroll region — `viewport` is the window's
    /// inner size, used for the grid's column count and the windowed row
    /// count.
    pub fn view(&self, viewport: Size) -> Element<'_, Message> {
        // The banner is optional, so the column takes its children one at a
        // time: a missing hero must not leave a spacing gap behind.
        let mut page = Column::new().push(self.header());
        if let Some(hero) = self.hero() {
            page = page.push(hero);
        }
        let page = page.push(self.quick_actions()).push(self.instances_section(viewport));

        scrollbar::scroll(
            page.spacing(24)
                .padding(Padding::new(0.0).left(28).right(28).top(24).bottom(28))
                .width(Fill),
        )
        .width(Fill)
        .height(Fill)
        .on_scroll(|scroll| Message::Scrolled(scroll.absolute_offset().y))
        .into()
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
