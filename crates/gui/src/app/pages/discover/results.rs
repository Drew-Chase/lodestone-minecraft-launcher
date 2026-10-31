//! The results: the grid, the compact rows or the table, the action each
//! project offers, and the lines that say why there is nothing to show.

use aurora_leaf::Icon;
use iced::widget::{Text, column, container, row};
use iced::{Element, Fill, Padding, Size, alignment};
use lodestone_platforms::{ContentType, SearchResult};

use super::card::{compact_row, glyph, grid_card, panel, table_header, table_row};
use super::{Discover, DiscoverView, InstallState, Message, Source};
use crate::app::components::button::Variant;
use crate::app::components::{TextButton, grid};
use crate::app::lang;
use crate::app::theme::{color, fonts};

/// The width the page keeps clear of the sidebar and its margins.
const CHROME_WIDTH: f32 = 68.0 + 56.0;
/// The narrowest a grid card may be.
const CARD_MIN_WIDTH: f32 = 280.0;
/// The space between cards and rows.
const GAP: f32 = 16.0;
/// The width at which compact rows sit two to a line.
const COMPACT_TWO_COLUMNS: f32 = 760.0;

/// The results in the chosen layout, with the line that explains an empty or failed search.
pub(super) fn view(page: &Discover, window: Size) -> Element<'_, Message> {
    let mut stack = column![].spacing(16).width(Fill);
    if let Some(line) = state_line(page) {
        stack = stack.push(line);
    }
    if !page.results.hits.is_empty() {
        stack = stack.push(match page.view {
            DiscoverView::Grid => grid_layout(page, window),
            DiscoverView::Compact => compact_layout(page, window),
            DiscoverView::Table => table_layout(page),
        });
    }
    stack.into()
}

/// The line under the filters: why a platform cannot be searched, the error of a failed
/// search, or that nothing matched.
fn state_line(page: &Discover) -> Option<Element<'_, Message>> {
    if !page.source.enabled() {
        return Some(unavailable(page.source));
    }
    let text = if let Some(error) = &page.results.error {
        lang::t_with("discover.error", &[("error", error.as_str())])
    } else if page.results.loading || !page.results.hits.is_empty() {
        return None;
    } else {
        lang::t("discover.empty").to_string()
    };
    Some(Text::new(text).size(13).font(fonts::Inter::REGULAR).color(color::Text::MUTED).into())
}

/// The explanation for a platform that cannot be searched here, with a way to its website.
fn unavailable(source: Source) -> Element<'static, Message> {
    let text = match source {
        Source::CurseForge => lang::t("discover.curseforge.no_key"),
        Source::Modrinth | Source::AtLauncher | Source::Technic | Source::Ftb => lang::t("discover.empty"),
    };
    let open = TextButton::content(
        Text::new(lang::t("discover.action.open_site"))
            .size(12)
            .font(fonts::Inter::MEDIUM)
            .color(color::Text::DEFAULT),
    )
    .variant(Variant::Ghost)
    .padding(Padding::new(0.0).left(12).right(12).top(7).bottom(7))
    .on_press(Message::Open(source.home_url().to_string()));
    row![Text::new(text).size(13).font(fonts::Inter::REGULAR).color(color::Text::MUTED), open,]
        .spacing(14)
        .align_y(alignment::Vertical::Center)
        .into()
}

/// The grid: as many cards a row as the width allows, each at least `CARD_MIN_WIDTH` wide.
fn grid_layout(page: &Discover, window: Size) -> Element<'_, Message> {
    let available = (window.width - CHROME_WIDTH).max(CARD_MIN_WIDTH);
    let columns = ((available + GAP) / (CARD_MIN_WIDTH + GAP)).floor().clamp(1.0, 4.0) as usize;
    let width = (available - GAP * (columns as f32 - 1.0)) / columns as f32;
    let cells = page
        .results
        .hits
        .iter()
        .map(|hit| grid_card(page.source, hit, width, page.artwork(hit.icon_url.as_deref()), action(page, hit)))
        .collect();
    grid::grid(cells, columns, GAP)
}

/// The compact rows: two to a line when the window is wide enough, one otherwise.
fn compact_layout(page: &Discover, window: Size) -> Element<'_, Message> {
    let available = (window.width - CHROME_WIDTH).max(320.0);
    let columns = if available >= COMPACT_TWO_COLUMNS { 2 } else { 1 };
    let cells = page
        .results
        .hits
        .iter()
        .map(|hit| compact_row(page.source, hit, page.artwork(hit.icon_url.as_deref()), action(page, hit)))
        .collect();
    grid::grid(cells, columns, GAP)
}

/// The table: a header row, then one row per project.
fn table_layout(page: &Discover) -> Element<'_, Message> {
    let rows = page
        .results
        .hits
        .iter()
        .map(|hit| table_row(page.source, hit, page.artwork(hit.icon_url.as_deref()), action(page, hit)));
    container(column![table_header(), column(rows)]).width(Fill).style(panel).into()
}

/// The button a project's card shows. A modpack installs, and then opens its instance. Any other
/// project downloads into its shared folder, and then says it is in the library.
fn action<'a>(page: &Discover, hit: &'a SearchResult) -> Element<'a, Message> {
    if page.content == ContentType::Modpack {
        return match page.install_of(&hit.id) {
            Some(InstallState::Installed(instance)) => primary(
                lang::t("discover.action.open_instance"),
                Icon::lucide().external_link(),
                Message::OpenInstance(instance.clone()),
            ),
            Some(InstallState::Queued) => disabled(lang::t("discover.action.queued")),
            Some(InstallState::Preparing) => disabled(lang::t("discover.action.preparing")),
            None => primary(
                lang::t("discover.action.install"),
                Icon::lucide().download(),
                Message::Install(hit.id.clone()),
            ),
        };
    }
    if page.in_library(&hit.id) {
        return disabled(lang::t("discover.action.in_library"));
    }
    if page.downloading(&hit.id) {
        return disabled(lang::t("discover.action.downloading"));
    }
    primary(
        lang::t("discover.action.download"),
        Icon::lucide().download(),
        Message::Download(hit.id.clone()),
    )
}

/// A filled button with an icon: the action a card is for.
fn primary<'a>(label: &'static str, markup: &'static str, message: Message) -> Element<'a, Message> {
    TextButton::content(
        row![
            glyph(markup, 12.0, color::Primary::ON),
            Text::new(label).size(12).font(fonts::Inter::BOLD).color(color::Primary::ON),
        ]
        .spacing(6)
        .align_y(alignment::Vertical::Center),
    )
    .variant(Variant::Primary)
    .padding(Padding::new(0.0).left(12).right(12).top(7).bottom(7))
    .on_press(message)
    .into()
}

/// A dimmed label standing in for a button that cannot be pressed now.
fn disabled<'a>(label: &'static str) -> Element<'a, Message> {
    TextButton::content(Text::new(label).size(12).font(fonts::Inter::MEDIUM).color(color::Text::MUTED))
        .variant(Variant::Ghost)
        .padding(Padding::new(0.0).left(12).right(12).top(7).bottom(7))
        .disabled(true)
        .into()
}
