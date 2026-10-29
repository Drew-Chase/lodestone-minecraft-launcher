//! The Library — the launcher's landing page: the most-recent instance
//! hero, the quick actions, and the instance list (Grid / Compact / Table
//! views, filter tabs, search). The WHOLE page scrolls as one — the
//! instance list is virtualized against that page scroll, so nothing is
//! trapped in its own viewport on short windows.

mod hero;
pub mod instance;
mod list;
mod toolbar;

use instance::{Instance, Loader};

use crate::app::components::instance_card::{CARD_HEIGHT, COMPACT_ROW_HEIGHT, TABLE_ROW_HEIGHT};
use crate::app::theme::fonts;
use iced::widget::{Text, column, container, scrollable, svg};
use iced::{Background, Border, Color, Element, Fill, Padding, Size};
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
    /// Play was pressed on an instance (dummy until the game launcher).
    Play(String),
    /// Open an instance's detail page — the root routes to it.
    OpenInstance(String),
    /// A per-instance menu button (folder/settings/kebab) — dummy.
    InstanceMenu { instance: String, action: &'static str },
    /// A header/quick action (New Instance, Import, ...) — dummy.
    QuickAction(&'static str),
    /// The refresh button — the root re-queries the database.
    Reload,
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
            LibraryView::Grid => "Grid",
            LibraryView::Compact => "Compact",
            LibraryView::Table => "Table",
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
            LibraryFilter::Recent => "Recent",
            LibraryFilter::Pinned => "Pinned",
            LibraryFilter::Modded => "Modded",
            LibraryFilter::Vanilla => "Vanilla",
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
}

impl Library {
    /// Applies a display-state change. Database effects (`Reload`) are
    /// intercepted by the root before reaching here. The scroll offset is
    /// owned by the page scroll alone — filter changes must not jump it.
    pub fn update(&mut self, message: Message) {
        match message {
            Message::SearchChanged(search) => self.search = search,
            Message::FilterChanged(filter) => self.filter = filter,
            Message::ViewChanged(view) => self.view = view,
            Message::Scrolled(offset) => self.scroll_y = offset.max(0.0),
            Message::Play(name) => info!("Play requested for {name} (dummy)"),
            Message::OpenInstance(id) => info!("Open instance {id} requested"),
            Message::InstanceMenu { instance, action } => info!("Instance '{instance}' menu: {action} (dummy)"),
            Message::QuickAction(action) => info!("Quick action '{action}' (dummy)"),
            Message::Reload => {} // handled by the root
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

    /// The whole page as one scroll region — `viewport` is the window's
    /// inner size, used for the grid's column count and the windowed row
    /// count.
    pub fn view(&self, viewport: Size) -> Element<'_, Message> {
        scrollable(
            column![self.header(), self.hero(), self.quick_actions(), self.instances_section(viewport),]
                .spacing(24)
                .padding(Padding::new(0.0).left(28).right(28).top(24).bottom(28))
                .width(Fill),
        )
        .width(Fill)
        .height(Fill)
        .on_scroll(|page| Message::Scrolled(page.absolute_offset().y))
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
