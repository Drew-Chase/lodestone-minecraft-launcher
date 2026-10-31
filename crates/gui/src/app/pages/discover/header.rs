//! The Discover header: the title and search field, the source selector, and
//! the content tabs.

use aurora_leaf::Icon;
use iced::widget::Id;
use iced::widget::{Space, Text, container, row, stack, text_input};
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
    row![
        iced::widget::column![
            Text::new(lang::t("discover.title"))
                .size(24)
                .font(fonts::Inter::EXTRA_BOLD)
                .color(color::Text::HEADLINE),
            Text::new(subtitle).size(12).font(fonts::Inter::REGULAR).color(color::Text::MUTED),
        ]
        .spacing(2),
        Space::new().width(Fill),
        search_box(page),
    ]
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
        container(glyph(Icon::lucide().search(), 14.0, color::Text::PLACEHOLDER))
            .width(Length::Fixed(SEARCH_WIDTH))
            .height(Fill)
            .align_x(alignment::Horizontal::Left)
            .align_y(alignment::Vertical::Center)
            .padding(Padding::new(0.0).left(14)),
        container(Text::new(shortcut).size(11).font(fonts::JetBrainsMono::REGULAR).color(color::Text::MUTED),)
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
            .color(color::Text::PLACEHOLDER),
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
        color::Text::DISABLED
    } else if active {
        color::Primary::DEFAULT
    } else {
        color::Text::MUTED
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
            background: if active { Some(Background::Color(color::Primary::TINT)) } else { None },
            border: iced::Border {
                radius: 999.0.into(),
                width: 1.0,
                color: if active { color::Primary::GLOW } else { color::Surface::LINE },
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
    Text::new(text)
        .size(11)
        .font(fonts::JetBrainsMono::REGULAR)
        .color(color::Text::PLACEHOLDER)
        .into()
}

/// The content tabs. A tab the platform does not host is dimmed and cannot be picked.
pub(super) fn tabs(page: &Discover) -> Element<'_, Message> {
    row(TABS.into_iter().map(|content| tab(page, content))).spacing(4).width(Fill).into()
}

/// One content tab, with an underline when it is the one shown.
fn tab(page: &Discover, content: ContentType) -> Element<'_, Message> {
    let supported = page.source.supports(content);
    let active = page.content == content;
    let ink = if !supported {
        color::Text::DISABLED
    } else if active {
        color::Primary::DEFAULT
    } else {
        color::Text::MUTED
    };
    let label = Text::new(tab_label(content))
        .size(13)
        .font(if active { fonts::Inter::SEMI_BOLD } else { fonts::Inter::MEDIUM })
        .color(ink);
    let button = TextButton::content(label)
        .variant(Variant::Tab)
        .padding(Padding::new(0.0).left(14).right(14).top(12).bottom(10))
        .disabled(!supported)
        .on_press(Message::ContentSelected(content));
    let underline = container(Space::new().width(Fill).height(2)).style(move |_| container::Style {
        background: if active {
            Some(Background::Color(color::Primary::DEFAULT))
        } else {
            None
        },
        ..container::Style::default()
    });
    iced::widget::column![button, underline].into()
}

/// The tab's label.
fn tab_label(content: ContentType) -> &'static str {
    match content {
        ContentType::Modpack => lang::t("discover.tab.modpacks"),
        ContentType::ResourcePack => lang::t("discover.tab.resource_packs"),
        ContentType::Shader => lang::t("discover.tab.shaders"),
        ContentType::World => lang::t("discover.tab.worlds"),
        ContentType::DataPack => lang::t("discover.tab.data_packs"),
        ContentType::Mod => lang::t("discover.tab.modpacks"),
    }
}
