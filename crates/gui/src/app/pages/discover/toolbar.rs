//! The filter row: the category, loader, version and sort selectors, the count
//! of results, and the layout switcher.

use iced::widget::{Space, Text, row};
use iced::{Element, Fill, Length, alignment};

use super::{Discover, DiscoverView, Message};
use crate::app::components::{Select, segmented};
use crate::app::lang;
use crate::app::theme::{color, fonts};
use lodestone_platforms::{Facets, SearchSort};

/// The width of each filter selector.
const SELECT_WIDTH: f32 = 176.0;

/// The filter selectors, the count and the layout switcher.
pub(super) fn view(page: &Discover) -> Element<'_, Message> {
    let mut controls: Vec<Element<'_, Message>> = Vec::new();
    if let Some(facets) = page.facets().filter(|_| page.source.has_filters()) {
        controls.extend(filters(page, facets));
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
    .into()
}

/// The category, loader and version selectors, each shown only when the platform has such filters.
fn filters<'a>(page: &'a Discover, facets: &'a Facets) -> Vec<Element<'a, Message>> {
    let mut selects = Vec::new();
    if !facets.categories.is_empty() {
        let selected = page
            .category
            .as_ref()
            .and_then(|id| facets.categories.iter().position(|facet| &facet.id == id))
            .map(|index| index + 1)
            .unwrap_or(0);
        let mut options = vec![lang::t("discover.filter.any_category").to_string()];
        options.extend(facets.categories.iter().map(|facet| facet.name.clone()));
        selects.push(select(options, selected, Message::CategorySelected));
    }
    if !facets.loaders.is_empty() {
        let selected = page
            .loader
            .as_ref()
            .and_then(|name| facets.loaders.iter().position(|loader| loader == name))
            .map(|index| index + 1)
            .unwrap_or(0);
        let mut options = vec![lang::t("discover.filter.any_loader").to_string()];
        options.extend(facets.loaders.iter().map(|loader| loader_label(loader)));
        selects.push(select(options, selected, Message::LoaderSelected));
    }
    if !facets.game_versions.is_empty() {
        let selected = page
            .game_version
            .as_ref()
            .and_then(|version| facets.game_versions.iter().position(|known| known == version))
            .map(|index| index + 1)
            .unwrap_or(0);
        let mut options = vec![lang::t("discover.filter.any_version").to_string()];
        options.extend(facets.game_versions.iter().cloned());
        selects.push(
            Select::new(options, Some(selected))
                .searchable()
                .on_select(Message::VersionSelected)
                .width(Length::Fixed(SELECT_WIDTH))
                .into(),
        );
    }
    selects
}

/// A selector over `options`, with `selected` lit and `message` sent with the picked index.
fn select<'a>(options: Vec<String>, selected: usize, message: fn(usize) -> Message) -> Element<'a, Message> {
    Select::new(options, Some(selected))
        .on_select(message)
        .width(Length::Fixed(SELECT_WIDTH))
        .into()
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

/// A loader's name as it is shown: `fabric` is `Fabric`, and `neoforge` is `NeoForge`.
fn loader_label(name: &str) -> String {
    match name {
        "neoforge" => "NeoForge".to_string(),
        other => {
            let mut chars = other.chars();
            match chars.next() {
                Some(first) => first.to_uppercase().chain(chars).collect(),
                None => String::new(),
            }
        }
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
        .color(color::Text::MUTED)
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
