//! The Friends page. Friends, parties and voice are coming soon, so the page shows its header, its controls as they will
//! look, and a banner in place of the list. Nothing here reads or sends anything.

use iced::widget::{Space, Text, column, container, row, text_input};
use iced::{Background, Border, Element, Fill, Padding, alignment};

use crate::app::components::badge::Badge;
use crate::app::components::panel::panel_with_padding;
use crate::app::components::{TextButton, input};
use crate::app::lang;
use crate::app::theme::{color, fonts};

/// The Friends page, drawn in its coming-soon state.
pub fn view<'a, Message: Clone + 'a>() -> Element<'a, Message> {
    let header = row![
        column![
            Text::new(lang::t("friends.title"))
                .size(24)
                .font(fonts::Inter::EXTRA_BOLD)
                .color(color::text_headline()),
            Text::new(lang::t("common.coming_soon"))
                .size(12)
                .font(fonts::Inter::REGULAR)
                .color(color::text_muted()),
        ]
        .spacing(2),
        Space::new().width(Fill),
        text_input(lang::t("friends.search_placeholder"), "")
            .style(input::style)
            .size(13)
            .padding(10)
            .width(240.0),
        TextButton::ghost(lang::t("friends.invite_party")).disabled(true),
        TextButton::primary(lang::t("friends.add_friend")).disabled(true),
    ]
    .spacing(12)
    .align_y(alignment::Vertical::Center)
    .width(Fill);
    let body = column![banner(), empty_list()].spacing(18).width(Fill);
    container(column![header, body].spacing(22).width(Fill))
        .padding(Padding::new(0.0).top(24).left(28).right(28).bottom(0))
        .width(Fill)
        .height(Fill)
        .into()
}

/// The banner that says friends are coming soon, with the badge that marks it.
fn banner<'a, Message: 'a>() -> Element<'a, Message> {
    let text = column![
        Text::new(lang::t("friends.banner.title"))
            .size(14)
            .font(fonts::Inter::SEMI_BOLD)
            .color(color::text_headline()),
        Text::new(lang::t("friends.banner.body"))
            .size(12)
            .font(fonts::Inter::REGULAR)
            .color(color::text_muted()),
    ]
    .spacing(4)
    .width(Fill);
    let content = row![
        Badge::new(lang::t("common.coming_soon"), color::violet_light(), color::violet_tint()).compact(),
        text,
    ]
    .spacing(14)
    .align_y(alignment::Vertical::Center);
    container(panel_with_padding(content, 16.0)).width(Fill).into()
}

/// The place the friends list will be: an empty panel that says there is nobody to show yet.
fn empty_list<'a, Message: 'a>() -> Element<'a, Message> {
    let line = Text::new(lang::t("friends.empty"))
        .size(13)
        .font(fonts::Inter::REGULAR)
        .color(color::text_muted());
    container(line)
        .padding(Padding::new(24.0))
        .width(Fill)
        .style(|_| container::Style {
            background: Some(Background::Color(color::surface_bg2())),
            border: Border {
                radius: 14.0.into(),
                width: 1.0,
                color: color::surface_line(),
            },
            ..container::Style::default()
        })
        .into()
}
