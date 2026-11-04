//! The results: the grid, the compact rows or the table, the action each
//! project offers, and the lines that say why there is nothing to show.

use aurora_leaf::Icon;
use iced::mouse;
use iced::widget::{Text, column, container, mouse_area, responsive, row};
use iced::{Background, Border, Element, Fill, Length, Padding, alignment};
use iced_color_extension::rgb;
use lodestone_platforms::{ContentType, SearchResult};

use super::card::{CARD_RADIUS, compact_row, glyph, grid_card, panel, skeleton_card, skeleton_row, skeleton_table_row, table_header, table_row};
use super::{Discover, DiscoverView, InstallState, Message, Source};
use crate::app::components::button::Variant;
use crate::app::components::hover_scale::HoverScale;
use crate::app::components::{TextButton, anim, grid};
use crate::app::lang;
use crate::app::theme::{color, fonts};

/// The narrowest a grid card may be.
const CARD_MIN_WIDTH: f32 = 280.0;
/// The space between cards and rows.
const GAP: f32 = 16.0;
/// The width at which compact rows sit two to a line.
const COMPACT_TWO_COLUMNS: f32 = 760.0;
/// How many rows of placeholders a layout shows while its results load: a screenful, or the rows a page adds.
const SKELETON_ROWS: usize = 3;

/// The results in the chosen layout, with the lines that explain an empty or failed search and a Mods tab with no
/// instance that takes mods.
pub(super) fn view(page: &Discover) -> Element<'_, Message> {
    let mut stack = column![].spacing(16).width(Fill);
    if let Some(line) = mods_note(page) {
        stack = stack.push(line);
    }
    if let Some(line) = state_line(page) {
        stack = stack.push(line);
    }
    // A search that is loading shows placeholders where its results will be, beneath any results already on screen.
    if !page.results.hits.is_empty() || page.results.loading {
        // The grid and the compact rows count their columns from the width their column gives them, so they are laid
        // out at that width rather than at the window's.
        let layout: Element<'_, Message> = match page.view {
            DiscoverView::Grid => responsive(move |size| grid_layout(page, size.width)).height(Length::Shrink).into(),
            DiscoverView::Compact => responsive(move |size| compact_layout(page, size.width)).height(Length::Shrink).into(),
            DiscoverView::Table => table_layout(page),
        };
        stack = stack.push(layout);
    }
    stack.into()
}

/// The line over the Mods tab's results when the instance it adds to runs vanilla Minecraft, which takes no mods.
fn mods_note(page: &Discover) -> Option<Element<'_, Message>> {
    if page.content != ContentType::Mod || page.target().is_none_or(|target| target.takes_mods()) {
        return None;
    }
    Some(
        Text::new(lang::t("discover.mods.vanilla"))
            .size(13)
            .font(fonts::Inter::REGULAR)
            .color(color::text_muted())
            .into(),
    )
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
    Some(Text::new(text).size(13).font(fonts::Inter::REGULAR).color(color::text_muted()).into())
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
            .color(color::text_default()),
    )
    .variant(Variant::Ghost)
    .padding(Padding::new(0.0).left(12).right(12).top(7).bottom(7))
    .on_press(Message::Open(source.home_url().to_string()));
    row![Text::new(text).size(13).font(fonts::Inter::REGULAR).color(color::text_muted()), open,]
        .spacing(14)
        .align_y(alignment::Vertical::Center)
        .into()
}

/// The grid: as many cards a row as `width` allows, each at least `CARD_MIN_WIDTH` wide.
fn grid_layout(page: &Discover, width: f32) -> Element<'_, Message> {
    let available = width.max(CARD_MIN_WIDTH);
    let columns = ((available + GAP) / (CARD_MIN_WIDTH + GAP)).floor().clamp(1.0, 4.0) as usize;
    let width = (available - GAP * (columns as f32 - 1.0)) / columns as f32;
    let cells = page
        .results
        .hits
        .iter()
        .map(|hit| {
            let card = grid_card(
                page.source,
                hit,
                width,
                page.artwork(hit.banner_url.as_deref()),
                page.artwork(hit.icon_url.as_deref()),
                action(page, hit),
            );
            hoverable(card, page, &hit.id)
        })
        .chain((0..pending(page) * columns).map(|_| skeleton_card(width)))
        .collect();
    grid::grid(cells, columns, GAP)
}

/// The compact rows: two to a line when `width` is wide enough, one otherwise.
fn compact_layout(page: &Discover, width: f32) -> Element<'_, Message> {
    let available = width.max(320.0);
    let columns = if available >= COMPACT_TWO_COLUMNS { 2 } else { 1 };
    let cells = page
        .results
        .hits
        .iter()
        .map(|hit| {
            pressable(
                page,
                compact_row(page.source, hit, page.artwork(hit.icon_url.as_deref()), action(page, hit)),
                &hit.id,
            )
        })
        .chain((0..pending(page) * columns).map(|_| skeleton_row()))
        .collect();
    grid::grid(cells, columns, GAP)
}

/// The table: a header row, then one row per project.
fn table_layout(page: &Discover) -> Element<'_, Message> {
    let rows = page
        .results
        .hits
        .iter()
        .map(|hit| {
            pressable(
                page,
                table_row(page.source, hit, page.artwork(hit.icon_url.as_deref()), action(page, hit)),
                &hit.id,
            )
        })
        .chain((0..pending(page) * 2).map(|_| skeleton_table_row()));
    container(column![table_header(), column(rows)]).width(Fill).style(panel).into()
}

/// How many rows of placeholders the layouts add beneath the results: a few while a search is loading, none otherwise.
fn pending(page: &Discover) -> usize {
    if page.results.loading { SKELETON_ROWS } else { 0 }
}

/// A card or row that opens its project's resource page when it is pressed. While the pointer is over it, it lifts a
/// little, so the row reads as the one a press opens. The buttons inside take their own presses.
fn pressable<'a>(page: &Discover, content: Element<'a, Message>, project_id: &str) -> Element<'a, Message> {
    let hovered = page.is_hovered(project_id);
    let area = mouse_area(content)
        .on_press(Message::OpenResource(project_id.to_string()))
        .on_enter(Message::Hover {
            project_id: project_id.to_string(),
            hovered: true,
        })
        .on_exit(Message::Hover {
            project_id: project_id.to_string(),
            hovered: false,
        })
        .interaction(mouse::Interaction::Pointer);
    container(area).width(Fill).style(move |_| lift(hovered)).into()
}

/// The fill of a row under the pointer: a faint white wash, with its corners rounded.
fn lift(hovered: bool) -> container::Style {
    container::Style {
        background: hovered.then_some(Background::Color(rgb!(255, 255, 255, 4%))),
        border: Border {
            radius: 12.0.into(),
            ..Border::default()
        },
        ..container::Style::default()
    }
}

/// A grid card that opens its project's resource page when it is pressed. While the pointer is over it, it grows and
/// lights a ring as a library card does. The buttons inside take their own presses.
fn hoverable<'a>(card: Element<'a, Message>, page: &Discover, project_id: &str) -> Element<'a, Message> {
    let pressable = mouse_area(card)
        .on_press(Message::OpenResource(project_id.to_string()))
        .on_enter(Message::Hover {
            project_id: project_id.to_string(),
            hovered: true,
        })
        .on_exit(Message::Hover {
            project_id: project_id.to_string(),
            hovered: false,
        })
        .interaction(mouse::Interaction::Pointer);
    HoverScale::new(pressable, page.is_hovered(project_id))
        .ring(CARD_RADIUS, anim::fade(color::primary(), 0.6))
        .into()
}

/// The button a project's card shows. A mod is added to the instance the Mods tab picked. A modpack
/// installs, and then opens its instance. Any other project downloads into its shared folder, and then
/// says it is in the library.
fn action<'a>(page: &Discover, hit: &'a SearchResult) -> Element<'a, Message> {
    if page.content == ContentType::Mod {
        return mod_action(page, hit);
    }
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

/// The button a mod's card shows: it adds the mod to the instance the Mods tab picked, and then says it is there.
/// Without an instance that takes mods, the button stands dimmed.
fn mod_action<'a>(page: &Discover, hit: &'a SearchResult) -> Element<'a, Message> {
    if page.added_to_target(&hit.id) {
        return disabled(lang::t("discover.action.added"));
    }
    if page.adding_to_target(&hit.id) {
        return disabled(lang::t("discover.action.adding"));
    }
    match page.target() {
        Some(target) if target.takes_mods() => primary(lang::t("discover.action.add"), Icon::lucide().plus(), Message::AddMod(hit.id.clone())),
        _ => disabled(lang::t("discover.action.add")),
    }
}

/// A filled button with an icon: the action a card is for.
fn primary<'a>(label: &'static str, markup: &'static str, message: Message) -> Element<'a, Message> {
    TextButton::content(
        row![
            glyph(markup, 12.0, color::primary_on()),
            Text::new(label).size(12).font(fonts::Inter::BOLD).color(color::primary_on()),
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
    TextButton::content(Text::new(label).size(12).font(fonts::Inter::MEDIUM).color(color::text_muted()))
        .variant(Variant::Ghost)
        .padding(Padding::new(0.0).left(12).right(12).top(7).bottom(7))
        .disabled(true)
        .into()
}
