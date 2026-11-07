//! The Discover sidebar: the filters the platform offers for the tab on screen, beside the results. Every section is drawn
//! from the platform's own lists, so a platform that offers no filter shows no section. The sidebar keeps its place while
//! the results scroll, and its sections scroll inside it under a scrollbar of their own.

use aurora_leaf::Icon;
use iced::widget::{Space, Text, button, column, container, row, text_input};
use iced::{Background, Border, Color, Element, Fill, Length, Padding, alignment};
use lodestone_platforms::{CategoryGroup, Facets, Side};

use super::card::{CARD_RADIUS, glyph, panel};
use super::filters::{FilterMessage, Kind};
use super::{BOTTOM, Discover, GAP, Message, SIDE};
use crate::app::components::checkbox::{self, State};
use crate::app::components::{input, scrollbar};
use crate::app::lang;
use crate::app::theme::{color, fonts};

/// The width of a section's card. The scrollbar's track sits beside the cards, in the width `scrollbar::TRACK_WIDTH` takes.
const WIDTH: f32 = 240.0;
/// The space between the sidebar's scrollbar and the results.
const COLUMN_GAP: f32 = 16.0;
/// How many choices a section lists before its "Show more" switch.
const SHOWN: usize = 8;
/// How many Minecraft versions the version section lists before its "Show more" switch.
const SHOWN_VERSIONS: usize = 10;
/// The space between two sections' cards.
const SECTION_GAP: f32 = 10.0;
/// The space between a section's title and its rows.
const TITLE_GAP: f32 = 6.0;
/// The padding inside a section's card, which its title and rows sit inside.
const CARD_PAD: f32 = 8.0;
/// The padding either side of a row's label, measured from the row's own edge.
const ROW_PAD_X: f32 = 8.0;
/// The padding above and below a row's label.
const ROW_PAD_Y: f32 = 7.0;
/// The corner radius of a row. It is the card's radius less the card's padding, so the two corners are concentric.
const ROW_RADIUS: f32 = CARD_RADIUS - CARD_PAD;

/// Whether the platform offers any filter at all.
fn has_sections(facets: &Facets) -> bool {
    !facets.game_versions.is_empty()
        || !facets.snapshot_versions.is_empty()
        || facets.loaders.len() > 1
        || !facets.category_groups.is_empty()
        || facets.environments
        || facets.open_source
        || facets.exclusions
}

/// The sidebar, or nothing when the platform offers no filter for the tab. It runs beside the scrolling part of the page,
/// from the top gap down to the foot of the results, and its sections scroll inside it.
pub(super) fn view(page: &Discover) -> Option<Element<'_, Message>> {
    let facets = page.facets().filter(|facets| has_sections(facets))?;
    let mut sections = column![].spacing(SECTION_GAP).width(Fill);
    if !facets.game_versions.is_empty() || !facets.snapshot_versions.is_empty() {
        sections = sections.push(versions(page, facets));
    }
    if facets.loaders.len() > 1 {
        sections = sections.push(loaders(page, facets));
    }
    for group in &facets.category_groups {
        sections = sections.push(category(page, group));
    }
    if facets.environments {
        sections = sections.push(environments(page));
    }
    if facets.open_source {
        sections = sections.push(license(page));
    }
    if facets.exclusions {
        sections = sections.push(exclusions(page, facets));
    }
    if !page.filters.is_empty() {
        sections = sections.push(clear());
    }
    // The cards end where the scrollbar's track begins, so the track floats over the padding and never over a card.
    let cards = container(sections).padding(Padding::new(0.0).right(scrollbar::TRACK_WIDTH)).width(Fill);
    let scrolled = scrollbar::scroll(cards).width(Length::Fixed(WIDTH + scrollbar::TRACK_WIDTH)).height(Fill);
    let margins = Padding::new(0.0).top(GAP).bottom(BOTTOM).left(SIDE).right(COLUMN_GAP);
    Some(container(scrolled).padding(margins).height(Fill).into())
}

/// One choice in a list: what it shows, whether it is picked, and what pressing it sends.
struct Choice {
    label: String,
    picked: bool,
    message: Message,
}

/// A section's card: its title over its body.
fn section<'a>(title: String, body: impl Into<Element<'a, Message>>) -> Element<'a, Message> {
    let title = container(Text::new(title).size(12).font(fonts::Inter::SEMI_BOLD).color(color::text_muted()))
        .padding(Padding::new(0.0).top(4.0).left(ROW_PAD_X).right(ROW_PAD_X));
    card(column![title, body.into()].spacing(TITLE_GAP).width(Fill))
}

/// A card that holds a section's title and rows. It is the panel the results use, and its rows sit inside its padding.
fn card<'a>(body: impl Into<Element<'a, Message>>) -> Element<'a, Message> {
    container(body).padding(CARD_PAD).width(Fill).style(panel).into()
}

/// One row of a sidebar list: a box that shows whether it is picked, and the label. The whole row is the button.
fn option<'a>(label: String, picked: bool, message: Message) -> Element<'a, Message> {
    let state = if picked { State::Checked } else { State::Unchecked };
    let ink = if picked { color::text_headline() } else { color::text_default() };
    let content = row![checkbox::mark(state), Text::new(label).size(13).font(fonts::Inter::REGULAR).color(ink)]
        .spacing(10)
        .align_y(alignment::Vertical::Center);
    row_button(content, picked, message)
}

/// A pressable row, padded so its content does not touch its edge. The row lightens under the pointer. A picked row is
/// tinted in the accent, and that tint deepens under the pointer.
fn row_button<'a>(content: impl Into<Element<'a, Message>>, picked: bool, message: Message) -> Element<'a, Message> {
    button(container(content).width(Fill))
        .width(Fill)
        .padding(Padding::new(0.0).top(ROW_PAD_Y).bottom(ROW_PAD_Y).left(ROW_PAD_X).right(ROW_PAD_X))
        .style(move |_, status| row_style(picked, status))
        .on_press(message)
        .into()
}

/// The look of a row in `status`. An idle row has no fill, so its card shows through.
fn row_style(picked: bool, status: button::Status) -> button::Style {
    let hovered = matches!(status, button::Status::Hovered | button::Status::Pressed);
    let background = match (picked, hovered) {
        (true, true) => Color { a: 0.18, ..color::primary() },
        (true, false) => color::primary_tint(),
        (false, true) => color::surface_bg4(),
        (false, false) => return button::Style::default(),
    };
    button::Style {
        background: Some(Background::Color(background)),
        text_color: color::text_default(),
        border: Border {
            radius: ROW_RADIUS.into(),
            ..Border::default()
        },
        ..button::Style::default()
    }
}

/// A list of choices: the first `shown` of them, and a switch that shows the rest. The list is named `section`, so its
/// switch is remembered by the panel.
fn list<'a>(page: &Discover, section: &str, choices: Vec<Choice>, shown: usize) -> Element<'a, Message> {
    let expanded = page.panel.expanded.contains(section);
    let total = choices.len();
    let limit = if expanded { total } else { shown.min(total) };
    let mut rows = column![].spacing(2).width(Fill);
    for choice in choices.into_iter().take(limit) {
        rows = rows.push(option(choice.label, choice.picked, choice.message));
    }
    if total > shown {
        rows = rows.push(show_more(section, expanded, total - shown));
    }
    rows.into()
}

/// The switch under a list that is cut short, or that shows every item and can cut itself short again. Its label sits in
/// the middle of the row.
fn show_more<'a>(section: &str, expanded: bool, hidden: usize) -> Element<'a, Message> {
    let (label, next) = if expanded {
        (lang::t("discover.sidebar.show_fewer").to_string(), false)
    } else {
        let count = hidden.to_string();
        (lang::t_with("discover.sidebar.show_more", &[("count", count.as_str())]).to_string(), true)
    };
    let label = Text::new(label).size(12).font(fonts::Inter::MEDIUM).color(color::text_muted());
    let centred = container(label).width(Fill).align_x(alignment::Horizontal::Center);
    row_button(
        centred,
        false,
        Message::Filter(FilterMessage::Expand {
            section: section.to_string(),
            expanded: next,
        }),
    )
}

/// The Minecraft versions: a search over them, the release versions, and a switch that adds the others.
fn versions<'a>(page: &'a Discover, facets: &'a Facets) -> Element<'a, Message> {
    let search = text_input(lang::t("discover.sidebar.search_versions"), &page.panel.version_search)
        .on_input(|text| Message::Filter(FilterMessage::VersionSearch(text)))
        .style(input::style)
        .padding(Padding::new(0.0).left(10).right(10).top(7).bottom(7))
        .size(12)
        .width(Fill);
    let mut pool: Vec<&String> = facets.game_versions.iter().collect();
    if page.panel.show_all_versions {
        pool.extend(facets.snapshot_versions.iter());
    }
    let needle = page.panel.version_search.to_lowercase();
    let choices: Vec<Choice> = pool
        .into_iter()
        .filter(|version| version.to_lowercase().contains(&needle))
        .map(|version| Choice {
            label: version.clone(),
            picked: page.filters.game_versions.contains(version),
            message: Message::Filter(FilterMessage::Pick(Kind::GameVersion, version.clone())),
        })
        .collect();
    let mut body = column![search, list(page, "versions", choices, SHOWN_VERSIONS)].spacing(8).width(Fill);
    if !facets.snapshot_versions.is_empty() {
        let all = page.panel.show_all_versions;
        body = body.push(option(
            lang::t("discover.sidebar.show_all_versions").to_string(),
            all,
            Message::Filter(FilterMessage::ShowAllVersions(!all)),
        ));
    }
    section(lang::t("discover.sidebar.game_version").to_string(), body)
}

/// The loaders most mods and modpacks run on. They lead the loader lists, and the rest keep the platform's order.
const COMMON_LOADERS: [&str; 4] = ["fabric", "forge", "neoforge", "quilt"];

/// The platform's loaders, with the common ones first.
fn ordered_loaders(facets: &Facets) -> Vec<&String> {
    let mut loaders: Vec<&String> = facets.loaders.iter().collect();
    loaders.sort_by_key(|name| {
        COMMON_LOADERS
            .iter()
            .position(|common| *common == name.as_str())
            .unwrap_or(COMMON_LOADERS.len())
    });
    loaders
}

/// The mod loaders, as the platform names them.
fn loaders<'a>(page: &'a Discover, facets: &'a Facets) -> Element<'a, Message> {
    let choices = ordered_loaders(facets)
        .into_iter()
        .map(|name| Choice {
            label: loader_label(name),
            picked: page.filters.loaders.contains(name),
            message: Message::Filter(FilterMessage::Pick(Kind::Loader, name.clone())),
        })
        .collect();
    section(lang::t("discover.sidebar.loader").to_string(), list(page, "loaders", choices, SHOWN))
}

/// One header's categories, titled by the header.
fn category<'a>(page: &'a Discover, group: &'a CategoryGroup) -> Element<'a, Message> {
    let choices = group
        .categories
        .iter()
        .map(|facet| Choice {
            label: facet.name.clone(),
            picked: page.filters.categories.contains(&facet.id),
            message: Message::Filter(FilterMessage::Pick(Kind::Category, facet.id.clone())),
        })
        .collect();
    let key = format!("category:{}", group.header);
    section(header_label(&group.header), list(page, &key, choices, SHOWN))
}

/// The sides a project runs on. Each picked side is required.
fn environments<'a>(page: &'a Discover) -> Element<'a, Message> {
    let mut rows = column![].spacing(2).width(Fill);
    for (side, label) in [
        (Side::Client, lang::t("discover.sidebar.client")),
        (Side::Server, lang::t("discover.sidebar.server")),
    ] {
        let picked = page.filters.environments.contains(&side);
        rows = rows.push(option(label.to_string(), picked, Message::Filter(FilterMessage::Environment(side))));
    }
    section(lang::t("discover.sidebar.environment").to_string(), rows)
}

/// The licence filter: only open-source projects.
fn license<'a>(page: &'a Discover) -> Element<'a, Message> {
    let open = page.filters.open_source;
    let body = option(
        lang::t("discover.sidebar.open_source").to_string(),
        open,
        Message::Filter(FilterMessage::OpenSource(!open)),
    );
    section(lang::t("discover.sidebar.license").to_string(), body)
}

/// The exclusions: a collapsed header that opens to the categories and loaders a project must not have.
fn exclusions<'a>(page: &'a Discover, facets: &'a Facets) -> Element<'a, Message> {
    let open = page.panel.expanded.contains("exclusions");
    let chevron = if open {
        Icon::lucide().chevron_down()
    } else {
        Icon::lucide().chevron_right()
    };
    let head = row_button(
        row![
            Text::new(lang::t("discover.sidebar.exclusions").to_string())
                .size(12)
                .font(fonts::Inter::SEMI_BOLD)
                .color(color::text_muted()),
            Space::new().width(Fill),
            glyph(chevron, 12.0, color::text_muted()),
        ]
        .align_y(alignment::Vertical::Center)
        .width(Fill),
        false,
        Message::Filter(FilterMessage::Expand {
            section: "exclusions".to_string(),
            expanded: !open,
        }),
    );
    if !open {
        return card(head);
    }
    let categories = facets
        .category_groups
        .iter()
        .flat_map(|group| group.categories.iter())
        .map(|facet| Choice {
            label: facet.name.clone(),
            picked: page.filters.excluded_categories.contains(&facet.id),
            message: Message::Filter(FilterMessage::Exclude(Kind::Category, facet.id.clone())),
        })
        .collect();
    let mut body = column![head, list(page, "exclusion-categories", categories, SHOWN)]
        .spacing(TITLE_GAP)
        .width(Fill);
    if facets.loaders.len() > 1 {
        let loaders = ordered_loaders(facets)
            .into_iter()
            .map(|name| Choice {
                label: loader_label(name),
                picked: page.filters.excluded_loaders.contains(name),
                message: Message::Filter(FilterMessage::Exclude(Kind::Loader, name.clone())),
            })
            .collect();
        body = body.push(list(page, "exclusion-loaders", loaders, SHOWN));
    }
    card(body)
}

/// The button that clears every pick. It sits under the cards, inset to the rows' edges.
fn clear<'a>() -> Element<'a, Message> {
    let label = Text::new(lang::t("discover.sidebar.clear").to_string())
        .size(12)
        .font(fonts::Inter::MEDIUM)
        .color(color::text_default());
    let centred = container(label).width(Fill).align_x(alignment::Horizontal::Center);
    container(row_button(centred, false, Message::Filter(FilterMessage::Clear)))
        .padding(Padding::new(0.0).left(CARD_PAD).right(CARD_PAD))
        .width(Fill)
        .into()
}

/// The title of a category group, from the header the platform files it under.
fn header_label(header: &str) -> String {
    match header {
        "categories" => lang::t("discover.sidebar.header.categories").to_string(),
        "features" => lang::t("discover.sidebar.header.features").to_string(),
        "resolutions" => lang::t("discover.sidebar.header.resolutions").to_string(),
        "performance impact" => lang::t("discover.sidebar.header.performance_impact").to_string(),
        other => capitalize(other),
    }
}

/// A loader's name as it is shown: `fabric` is `Fabric`, and `neoforge` is `NeoForge`.
fn loader_label(name: &str) -> String {
    match name {
        "neoforge" => "NeoForge".to_string(),
        other => capitalize(other),
    }
}

/// `text` with the first letter of each word capitalised.
fn capitalize(text: &str) -> String {
    text.split(' ')
        .map(|word| {
            let mut chars = word.chars();
            match chars.next() {
                Some(first) => first.to_uppercase().chain(chars).collect::<String>(),
                None => String::new(),
            }
        })
        .collect::<Vec<_>>()
        .join(" ")
}

#[cfg(test)]
#[path = "../../../../tests/app/pages/discover/sidebar.rs"]
mod tests;
