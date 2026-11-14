//! The Discover step: a search of the modpacks Discover lists, and an install button for each hit. The install is the Discover
//! page's own: the root prepares it and queues it (see `app/setup.rs`).

use std::collections::HashSet;

use iced::widget::{Text, column, row, text_input};
use iced::{Element, Fill, Padding, alignment};
use lodestone_platforms::{SearchPage, SearchResult};

use crate::app::components::{TextButton, input};
use crate::app::install::InstallRequest;
use crate::app::lang;
use crate::app::theme::{color as palette, fonts};

/// How many modpacks the step lists for a search.
pub const LIMIT: u32 = 10;

/// The step's state: the search, its hits, and the installs the step has prepared or queued.
#[derive(Debug, Clone, Default)]
pub struct State {
    /// The search text. Empty browses the most popular modpacks.
    pub query: String,
    /// The hits of the last search that answered.
    pub hits: Vec<SearchResult>,
    /// Whether a search is running.
    pub searching: bool,
    /// Why the last search failed, when it did.
    pub error: Option<String>,
    /// The modpacks whose install is being prepared, by project id.
    pub preparing: HashSet<String>,
    /// The modpacks whose install this step queued, by project id.
    pub queued: HashSet<String>,
}

/// The step's messages.
#[derive(Debug, Clone)]
pub enum Message {
    /// The search text changed.
    Query(String),
    /// A search was asked for: Enter in the box.
    Search,
    /// The search answered, with its hits or the reason it failed.
    Results(Result<SearchPage, String>),
    /// The install of a hit was asked for, by project id.
    Install(String),
    /// The install of a hit was prepared, or could not be. The root queues it.
    Prepared(String, Result<Box<InstallRequest>, String>),
}

/// The step's body: the search box, the search's status, and a row for each hit with its install button.
pub fn view<'a>(state: &'a State) -> Element<'a, Message> {
    let search = text_input(lang::t("setup.discover.search_placeholder"), &state.query)
        .on_input(Message::Query)
        .on_submit(Message::Search)
        .size(14)
        .padding(Padding::new(0.0).left(14.0).right(14.0).top(11.0).bottom(11.0))
        .style(input::style)
        .width(Fill);
    let rows: Element<'a, Message> = if state.hits.is_empty() {
        status(state)
    } else {
        column(state.hits.iter().map(|hit| hit_row(hit, state))).spacing(12).width(Fill).into()
    };
    column![search, rows].spacing(16).width(Fill).into()
}

/// The line under the search box while there are no hits to list: a search under way, its failure, or that nothing matched.
fn status<'a>(state: &'a State) -> Element<'a, Message> {
    let (text, ink) = if state.searching {
        (lang::t("setup.discover.searching"), palette::text_muted())
    } else if let Some(error) = &state.error {
        return Text::new(error.clone()).size(12).font(fonts::Inter::MEDIUM).color(palette::pink()).into();
    } else {
        (lang::t("setup.discover.none"), palette::text_muted())
    };
    Text::new(text).size(12).font(fonts::Inter::REGULAR).color(ink).into()
}

/// One hit: its name, its author and its one-line description, and the button that installs it, or what became of the install.
fn hit_row<'a>(hit: &'a SearchResult, state: &'a State) -> Element<'a, Message> {
    let action: Element<'a, Message> = if state.queued.contains(&hit.id) {
        Text::new(lang::t("setup.discover.queued"))
            .size(12)
            .font(fonts::Inter::SEMI_BOLD)
            .color(palette::primary())
            .into()
    } else if state.preparing.contains(&hit.id) {
        Text::new(lang::t("setup.discover.preparing"))
            .size(12)
            .font(fonts::Inter::REGULAR)
            .color(palette::text_muted())
            .into()
    } else {
        TextButton::ghost(lang::t("setup.discover.install"))
            .padding(Padding::new(0.0).left(14.0).right(14.0).top(7.0).bottom(7.0))
            .on_press(Message::Install(hit.id.clone()))
            .into()
    };
    let detail = match &hit.author {
        Some(author) => format!("{author} · {}", hit.description),
        None => hit.description.clone(),
    };
    row![
        column![
            Text::new(hit.name.clone())
                .size(14)
                .font(fonts::Inter::SEMI_BOLD)
                .color(palette::text_headline()),
            Text::new(detail).size(12).font(fonts::Inter::REGULAR).color(palette::text_muted()),
        ]
        .spacing(3)
        .width(Fill),
        action,
    ]
    .spacing(12)
    .align_y(alignment::Vertical::Center)
    .into()
}
