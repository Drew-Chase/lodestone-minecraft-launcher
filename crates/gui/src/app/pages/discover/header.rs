//! The Discover header: the title and search field, the source selector, and
//! the content tabs.

use aurora_leaf::Icon;
use iced::widget::Id;
use iced::widget::{Row, Space, Text, container, row, stack, text_input};
use iced::{Background, Element, Fill, Length, Padding, alignment};

use super::card::glyph;
use super::{Discover, Message, SEARCH_ID, Source, TABS};
use crate::app::components::button::Variant;
use crate::app::components::{TextButton, input, platform_logo};
use crate::app::lang;
use crate::app::theme::{color, fonts};
use lodestone_platforms::ContentType;

/// The search field's width.
const SEARCH_WIDTH: f32 = 460.0;

/// The page title, with the platform in its subtitle, and the search field.
pub(super) fn view(page: &Discover) -> Element<'_, Message> {
    let subtitle = lang::t_with("discover.subtitle", &[("platform", page.source.name())]);
    let mut heading = iced::widget::Row::new().spacing(16).align_y(alignment::Vertical::Center);
    // A page opened from an instance to add mods to offers a way back to that instance.
    if let Some(origin) = page.origin() {
        let back: Element<'_, Message> = TextButton::labeled_icon(
            Icon::lucide().chevron_left(),
            lang::t_with("discover.back", &[("name", origin.name.as_str())]),
            Variant::Ghost,
        )
        .on_press(Message::Back)
        .into();
        heading = heading.push(back);
    }
    heading = heading.push(
        iced::widget::column![
            Text::new(lang::t("discover.title"))
                .size(24)
                .font(fonts::Inter::EXTRA_BOLD)
                .color(color::text_headline()),
            Text::new(subtitle).size(12).font(fonts::Inter::REGULAR).color(color::text_muted()),
        ]
        .spacing(2),
    );
    row![heading, Space::new().width(Fill), search_box(page)]
        .spacing(16)
        .align_y(alignment::Vertical::Center)
        .width(Fill)
        .into()
}

/// The search input, with its icon on the left and the Ctrl/⌘K hint on the right.
fn search_box(page: &Discover) -> Element<'_, Message> {
    let shortcut = if cfg!(target_os = "macos") {
        lang::t("discover.shortcut.mac")
    } else {
        lang::t("discover.shortcut.other")
    };
    stack![
        text_input(&page.placeholder, &page.text)
            .id(Id::new(SEARCH_ID))
            .on_input(Message::QueryChanged)
            .on_submit(Message::SearchNow)
            .style(input::style)
            .size(13)
            .padding(Padding::new(0.0).left(38).right(84).top(10).bottom(10))
            .width(Length::Fixed(SEARCH_WIDTH)),
        container(glyph(Icon::lucide().search(), 14.0, color::text_placeholder()))
            .width(Length::Fixed(SEARCH_WIDTH))
            .height(Fill)
            .align_x(alignment::Horizontal::Left)
            .align_y(alignment::Vertical::Center)
            .padding(Padding::new(0.0).left(14)),
        container(
            Text::new(shortcut)
                .size(11)
                .font(fonts::JetBrainsMono::REGULAR)
                .color(color::text_muted()),
        )
        .width(Length::Fixed(SEARCH_WIDTH))
        .height(Fill)
        .align_x(alignment::Horizontal::Right)
        .align_y(alignment::Vertical::Center)
        .padding(Padding::new(0.0).right(12)),
    ]
    .width(Length::Fixed(SEARCH_WIDTH))
    .into()
}

/// The source selector: one chip per platform, with the selected one lit. A
/// platform that cannot be searched in this build is shown dimmed and cannot be picked.
pub(super) fn source_row(page: &Discover) -> Element<'_, Message> {
    let chips = Source::ALL.into_iter().map(|source| source_chip(page, source));
    row![
        Text::new(lang::t("discover.source"))
            .size(10)
            .font(fonts::JetBrainsMono::SEMI_BOLD)
            .color(color::text_placeholder()),
        row(chips).spacing(8).align_y(alignment::Vertical::Center),
        Space::new().width(Fill),
        status(page),
    ]
    .spacing(16)
    .align_y(alignment::Vertical::Center)
    .width(Fill)
    .into()
}

/// One platform's chip.
fn source_chip(page: &Discover, source: Source) -> Element<'_, Message> {
    let active = page.source == source;
    let enabled = source.enabled();
    let ink = if !enabled {
        color::text_disabled()
    } else if active {
        color::primary()
    } else {
        color::text_muted()
    };
    let content = row![
        platform_logo::logo(source.logo(), 14.0),
        Text::new(source.name())
            .size(13)
            .font(if active { fonts::Inter::SEMI_BOLD } else { fonts::Inter::MEDIUM })
            .color(ink),
    ]
    .spacing(8)
    .align_y(alignment::Vertical::Center);
    let button = TextButton::content(content)
        .variant(Variant::Tab)
        .padding(Padding::new(0.0).left(12).right(12).top(7).bottom(7))
        .disabled(!enabled)
        .on_press(Message::SourceSelected(source));
    container(button)
        .style(move |_| container::Style {
            background: if active { Some(Background::Color(color::primary_tint())) } else { None },
            border: iced::Border {
                radius: 999.0.into(),
                width: 1.0,
                color: if active { color::primary_glow() } else { color::surface_line() },
            },
            ..container::Style::default()
        })
        .into()
}

/// What the last request said about itself: its latency, and whether the cache answered.
fn status(page: &Discover) -> Element<'_, Message> {
    let text = match (page.results.elapsed_ms, page.results.cached) {
        (Some(ms), true) => lang::t_with("discover.status.cached", &[("ms", ms.to_string().as_str())]),
        (Some(ms), false) => lang::t_with("discover.status.api", &[("ms", ms.to_string().as_str())]),
        (None, _) => String::new(),
    };
    // One line: the source row gives this text only the room the chips leave, so it never wraps into a column of fragments.
    Text::new(text)
        .size(11)
        .font(fonts::JetBrainsMono::REGULAR)
        .color(color::text_placeholder())
        .wrapping(iced::widget::text::Wrapping::None)
        .into()
}

/// The width of one content tab. It holds the longest label, "Resource Packs", with room either side.
const TAB_WIDTH: f32 = 152.0;

/// The content tabs. A tab the platform does not host is dimmed and cannot be picked.
pub(super) fn tabs(page: &Discover) -> Element<'_, Message> {
    // The tabs wrap onto a second row when the window cannot hold all of them beside each other.
    let tabs: Vec<Element<'_, Message>> = TABS.into_iter().map(|content| tab(page, content)).collect();
    container(Row::with_children(tabs).spacing(4).width(Fill).wrap().vertical_spacing(4))
        .width(Fill)
        .into()
}

/// One content tab: a fixed-width target with its label centred, and the 2px bar along its bottom edge when it is the
/// one shown. The bar lies over the button, so the whole tab is the target, bar included.
fn tab(page: &Discover, content: ContentType) -> Element<'_, Message> {
    let supported = page.source.supports(content);
    let active = page.content == content;
    let ink = if !supported {
        color::text_disabled()
    } else if active {
        color::primary()
    } else {
        color::text_muted()
    };
    let label = container(
        Text::new(tab_label(content))
            .size(13)
            .font(if active { fonts::Inter::SEMI_BOLD } else { fonts::Inter::MEDIUM })
            .color(ink),
    )
    .width(Fill)
    .align_x(alignment::Horizontal::Center);
    let button = TextButton::content(label)
        .variant(Variant::Tab)
        .width(Fill)
        .padding(Padding::new(0.0).left(14).right(14).top(12).bottom(10))
        .disabled(!supported)
        .on_press(Message::ContentSelected(content));
    let underline = container(Space::new().width(Fill).height(2)).style(move |_| container::Style {
        background: if active { Some(Background::Color(color::primary())) } else { None },
        ..container::Style::default()
    });
    let bar = container(underline).width(Fill).height(Fill).align_y(alignment::Vertical::Bottom);
    stack![button, bar].width(Length::Fixed(TAB_WIDTH)).into()
}

/// The tab's label.
fn tab_label(content: ContentType) -> &'static str {
    match content {
        ContentType::Modpack => lang::t("discover.tab.modpacks"),
        ContentType::ResourcePack => lang::t("discover.tab.resource_packs"),
        ContentType::Shader => lang::t("discover.tab.shaders"),
        ContentType::World => lang::t("discover.tab.worlds"),
        ContentType::DataPack => lang::t("discover.tab.data_packs"),
        ContentType::Mod => lang::t("discover.tab.mods"),
    }
}
