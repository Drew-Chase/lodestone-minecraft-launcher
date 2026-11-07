//! The toolbar over the results: the instance the results are added to, the sort selector, the count of results, and the
//! layout switcher. The filters are in the sidebar beside it. Once the page scrolls past the toolbar, the same row is
//! pinned over the results.

use aurora_leaf::Icon;
use iced::widget::{Space, Text, container, opaque, row};
use iced::{Background, Element, Fill, Length, Padding, alignment};

use super::{Discover, DiscoverView, GAP, Message, SIDE, sidebar, target};
use crate::app::components::{Select, scrollbar, segmented, select};
use crate::app::lang;
use crate::app::theme::{color, fonts};
use lodestone_platforms::{ContentType, SearchSort};

/// The width of the sort selector.
const SELECT_WIDTH: f32 = 176.0;
/// The width of the instance selector, which carries the instance's name.
const INSTANCE_SELECT_WIDTH: f32 = 220.0;

/// The height of the toolbar. It is the selectors' trigger height. The row keeps it wherever it sits, so a pinned row
/// leaves a gap of the same height behind.
pub(super) const HEIGHT: f32 = select::TRIGGER_HEIGHT;

/// The instance selector, the sort selector, the count and the layout switcher.
pub(super) fn view(page: &Discover) -> Element<'_, Message> {
    let mut controls: Vec<Element<'_, Message>> = Vec::new();
    if target::takes_target(page.content) {
        controls.push(instance_select(page));
    }
    if page.content == ContentType::DataPack {
        controls.push(world_select(page));
    }
    controls.push(sort_select(page));
    row![
        row(controls).spacing(8).align_y(alignment::Vertical::Center),
        Space::new().width(Fill),
        count(page),
        switcher(page),
    ]
    .spacing(16)
    .align_y(alignment::Vertical::Center)
    .width(Fill)
    .height(Length::Fixed(HEIGHT))
    .into()
}

/// The toolbar pinned over the results, once the page has scrolled past it. Its background is the page's, so the results
/// pass beneath it. It starts beside the sidebar, which stays where it is, and stops short of the scrollbar's track, which
/// stays clear. `opaque` stops presses over the bar, so no card beneath can be pressed through it. The wheel still reaches
/// the page.
pub(super) fn pinned(page: &Discover) -> Element<'_, Message> {
    let bar = container(view(page))
        .padding(Padding::new(GAP).left(0.0).right(SIDE - scrollbar::TRACK_WIDTH).bottom(GAP))
        .width(Fill)
        .style(|_| container::Style {
            background: Some(Background::Color(color::surface_application())),
            ..container::Style::default()
        });
    container(opaque(bar))
        .padding(Padding::new(0.0).left(SIDE + sidebar::width_taken(page)).right(scrollbar::TRACK_WIDTH))
        .width(Fill)
        .into()
}

/// A selector over `options`, with `selected` lit and `message` sent with the picked index.
fn select<'a>(options: Vec<String>, selected: usize, message: fn(usize) -> Message) -> Element<'a, Message> {
    Select::new(options, Some(selected))
        .on_select(message)
        .width(Length::Fixed(SELECT_WIDTH))
        .into()
}

/// The selector for the instance the results are added to. It lists the instances the library holds, and it is dimmed
/// when there are none.
fn instance_select(page: &Discover) -> Element<'_, Message> {
    let names: Vec<String> = page.instances.iter().map(|choice| choice.name.clone()).collect();
    let selected = page
        .target
        .as_ref()
        .and_then(|id| page.instances.iter().position(|choice| &choice.id == id));
    let picker = Select::new(names.clone(), selected)
        .leading_icon(Icon::lucide().package())
        .on_select(Message::InstanceSelected)
        .width(Length::Fixed(INSTANCE_SELECT_WIDTH));
    if names.is_empty() {
        picker.placeholder(lang::t("discover.mods.no_instances")).disabled(true).into()
    } else {
        picker.placeholder(lang::t("discover.mods.choose_instance")).into()
    }
}

/// The selector for the world the datapacks go to. It lists the worlds of the target instance, and it is dimmed until an
/// instance is picked, and while that instance has no worlds.
fn world_select(page: &Discover) -> Element<'_, Message> {
    let names: Vec<String> = page.worlds.iter().map(|world| world.name.clone()).collect();
    let selected = page
        .target_world
        .as_ref()
        .and_then(|folder| page.worlds.iter().position(|world| &world.folder == folder));
    let picker = Select::new(names.clone(), selected)
        .leading_icon(Icon::lucide().globe())
        .on_select(Message::WorldSelected)
        .width(Length::Fixed(INSTANCE_SELECT_WIDTH));
    if page.target().is_none() {
        picker
            .placeholder(lang::t("discover.placement.pick_instance_first"))
            .disabled(true)
            .into()
    } else if names.is_empty() {
        picker.placeholder(lang::t("discover.placement.no_worlds")).disabled(true).into()
    } else {
        picker.placeholder(lang::t("discover.placement.choose_world")).into()
    }
}

/// The sort selector, offering the sorts the platform can express.
fn sort_select(page: &Discover) -> Element<'_, Message> {
    let sorts = page.source.sorts();
    let options: Vec<String> = sorts.iter().map(|sort| sort_label(*sort).to_string()).collect();
    let selected = sorts.iter().position(|sort| *sort == page.sort).unwrap_or(0);
    select(options, selected, Message::SortSelected)
}

/// A sort's label.
fn sort_label(sort: SearchSort) -> &'static str {
    match sort {
        SearchSort::Relevance => lang::t("discover.sort.relevance"),
        SearchSort::Popular => lang::t("discover.sort.popular"),
        SearchSort::Updated => lang::t("discover.sort.updated"),
        SearchSort::Newest => lang::t("discover.sort.newest"),
    }
}

/// How many results the search has, and where they come from.
fn count(page: &Discover) -> Element<'_, Message> {
    let text = if page.results.loading && page.results.hits.is_empty() {
        lang::t("discover.loading").to_string()
    } else {
        let shown = if page.results.total > 0 {
            page.results.total
        } else {
            page.results.hits.len() as u64
        };
        let number = super::card::short_count(shown);
        lang::t_with("discover.results", &[("count", number.as_str()), ("platform", page.source.name())])
    };
    Text::new(text)
        .size(12)
        .font(fonts::JetBrainsMono::REGULAR)
        .color(color::text_muted())
        .into()
}

/// The layout switcher: grid, compact or table.
fn switcher(page: &Discover) -> Element<'_, Message> {
    let options = [
        (DiscoverView::Grid, lang::t("discover.view.grid")),
        (DiscoverView::Compact, lang::t("discover.view.compact")),
        (DiscoverView::Table, lang::t("discover.view.table")),
    ];
    segmented::segmented(&options, page.view, Message::ViewSelected)
}
