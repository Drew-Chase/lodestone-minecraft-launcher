//! The Co-op Sync dialog: mirror a friend's instance, match its mods and shaders, then join their world. Co-op Sync is coming
//! soon, so the dialog keeps its layout and shows that nothing can be synced yet. Nothing here reads or sends anything.

use aurora_leaf::Icon;
use iced::widget::{Space, Text, column, container, row};
use iced::{Background, Border, Color, Element, Fill, Padding, alignment};

use crate::app::components::badge::Badge;
use crate::app::components::modal::Modal;
use crate::app::components::panel::panel_with_padding;
use crate::app::components::{TextButton, typography};
use crate::app::lang;
use crate::app::theme::{color, fonts};

/// The width of the dialog.
const WIDTH: f32 = 720.0;

/// What the dialog's controls send.
#[derive(Debug, Clone)]
pub enum Message {
    /// The dialog was closed, by its ✕, its Cancel button, or Escape.
    Close,
}

/// The dialog on screen: its banner, its three figures, the friends online, and the button that would sync.
pub fn view<'a>() -> Element<'a, Message> {
    let footer = row![
        Space::new().width(Fill),
        TextButton::ghost(lang::t("common.cancel")).on_press(Message::Close),
        TextButton::primary(lang::t("coop.sync_join")).disabled(true),
    ]
    .spacing(12)
    .align_y(alignment::Vertical::Center);
    Modal::new(lang::t("coop.title"), body())
        .subtitle(lang::t("coop.subtitle"))
        .icon(Icon::lucide().users(), color::violet_light(), color::violet_tint())
        .footer(footer)
        .on_close(Message::Close)
        .width(WIDTH)
        .into()
}

/// The dialog's body: the banner, the three figures, and the list of friends online.
fn body<'a>() -> Element<'a, Message> {
    column![banner(), figures(), online_now()].spacing(20).width(Fill).into()
}

/// The banner that says Co-op Sync is coming soon.
fn banner<'a>() -> Element<'a, Message> {
    let text = column![
        Text::new(lang::t("coop.banner.title"))
            .size(14)
            .font(fonts::Inter::SEMI_BOLD)
            .color(color::text_headline()),
        Text::new(lang::t("coop.banner.body"))
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
    panel_with_padding(content, 14.0)
}

/// The three figures across the top: friends online, instances shared, and the average sync time. None has a value yet.
fn figures<'a>() -> Element<'a, Message> {
    row![
        figure(lang::t("coop.figure.friends"), color::primary()),
        figure(lang::t("coop.figure.instances"), color::violet_light()),
        figure(lang::t("coop.figure.sync_time"), color::cyan()),
    ]
    .spacing(14)
    .width(Fill)
    .into()
}

/// One figure: its label, and a dash where its value will be.
fn figure<'a>(label: &'static str, ink: Color) -> Element<'a, Message> {
    let content = column![
        typography::eyebrow(label),
        Text::new("—").size(26).font(fonts::Inter::EXTRA_BOLD).color(ink),
    ]
    .spacing(8)
    .width(Fill);
    container(panel_with_padding(content, 16.0)).width(Fill).into()
}

/// The friends online, with a line saying there are none to sync with yet.
fn online_now<'a>() -> Element<'a, Message> {
    let line = Text::new(lang::t("coop.empty"))
        .size(13)
        .font(fonts::Inter::REGULAR)
        .color(color::text_muted());
    column![
        typography::eyebrow(lang::t("coop.online_now")),
        container(line).padding(Padding::new(20.0)).width(Fill).style(|_| container::Style {
            background: Some(Background::Color(color::surface_bg2())),
            border: Border {
                radius: 14.0.into(),
                width: 1.0,
                color: color::surface_line(),
            },
            ..container::Style::default()
        }),
    ]
    .spacing(10)
    .width(Fill)
    .into()
}
