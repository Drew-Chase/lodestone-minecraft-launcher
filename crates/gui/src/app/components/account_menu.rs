//! The account menu: the player's face, which opens a popover of the signed-in accounts. A press on an account switches
//! to it at once, and the last row signs a Microsoft account in. The face sits at the foot of the sidebar, or in the
//! floating navigation bar.
//!
//! The face is the popover's trigger, so the hint that names it wraps the popover from outside, as the runs indicator does.

use aurora_leaf::Icon;
use iced::widget::image::Handle;
use iced::widget::{Space, button, column, container, image, row, stack, text};
use iced::{Background, Border, Color, Element, Fill, Padding, alignment};
use iced_color_extension::rgb;

use crate::app::components::TextButton;
use crate::app::components::button::Variant;
use crate::app::components::popover::{Placement, Popover};
use crate::app::face::AVATAR_RADIUS;
use crate::app::lang;
use crate::app::theme::{Accent, color, fonts, utility};

/// The side of the face in the trigger, in pixels.
const FACE_SIZE: f32 = 40.0;
/// The side of an account's avatar in the panel, in pixels.
const ROW_AVATAR_SIZE: f32 = 32.0;
/// The width of the panel, in pixels.
const PANEL_WIDTH: f32 = 260.0;
/// The colors an account without a face takes its initial on, cycled by its place in the list.
const AVATAR_ACCENTS: [Accent; 5] = [Accent::Green, Accent::Violet, Accent::Cyan, Accent::Amber, Accent::Pink];

/// What the menu's controls send. The owner maps each one into its own message type with the function it passes.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Message {
    /// Opens or shuts the menu.
    Toggle,
    /// Shuts the menu, from a press outside it or Escape.
    Dismiss,
    /// Makes the account at this index the one that plays.
    Switch(usize),
    /// Signs a Microsoft account in.
    Add,
}

/// One signed-in account as the menu lists it.
#[derive(Debug, Clone)]
pub struct Entry {
    pub username: String,
    pub microsoft: bool,
    pub playing: bool,
    /// The account's face, when it is loaded. Only the account that plays has one.
    pub face: Option<Handle>,
}

/// The menu's state: whether the panel is open, and the accounts it lists.
pub struct Menu {
    pub open: bool,
    pub entries: Vec<Entry>,
}

/// The player's face that opens the menu. `face`, `username` and `online` draw it, as the sidebar's face always has.
/// `placement` says where the panel opens, and `to` maps the menu's messages into the owner's.
pub fn trigger<M: Clone + 'static>(
    face: Option<Handle>,
    username: Option<String>,
    online: bool,
    menu: Menu,
    placement: Placement,
    to: fn(Message) -> M,
) -> Element<'static, M> {
    Popover::new(face_tile(face, username, online), panel(menu.entries, to))
        .placement(placement)
        .open(menu.open)
        .on_toggle(to(Message::Toggle))
        .on_dismiss(to(Message::Dismiss))
        .into()
}

/// The face: the 8×8 face upscaled to 40×40 with the presence dot pinned to its bottom-right corner. Falls back to the
/// username's initial on the green gradient when no face is loaded.
fn face_tile<M: 'static>(face: Option<Handle>, username: Option<String>, online: bool) -> Element<'static, M> {
    let face_layer: Element<'static, M> = match face {
        Some(handle) => image(handle).width(FACE_SIZE).height(FACE_SIZE).into(),
        None => {
            let initial = username
                .as_deref()
                .and_then(|name| name.chars().next())
                .map(|initial| initial.to_uppercase().to_string())
                .unwrap_or_default();
            container(text(initial).size(16).font(fonts::Inter::EXTRA_BOLD).color(color::primary_on()))
                .width(Fill)
                .height(Fill)
                .align_x(alignment::Horizontal::Center)
                .align_y(alignment::Vertical::Center)
                .into()
        }
    };

    let dot_ink = if online { color::primary() } else { color::text_disabled() };

    container(
        stack![
            face_layer,
            // The ring rides on top of the face; the corners themselves are faded in the texture.
            container(Space::new().width(Fill).height(Fill)).style(|_| container::Style {
                border: Border {
                    radius: (AVATAR_RADIUS + 2.0).into(),
                    width: 1.5,
                    color: rgb!(255, 255, 255, 25%),
                },
                ..container::Style::default()
            }),
            container(container(Space::new().width(10).height(10)).style(move |_| container::Style {
                background: Some(Background::Color(dot_ink)),
                border: Border {
                    radius: 5.0.into(),
                    width: 2.0,
                    color: color::surface_bg1(),
                },
                ..container::Style::default()
            }))
            .width(Fill)
            .height(Fill)
            .align_x(alignment::Horizontal::Right)
            .align_y(alignment::Vertical::Bottom)
            .padding(Padding::new(0.0).right(1).bottom(1)),
        ]
        .width(FACE_SIZE)
        .height(FACE_SIZE),
    )
    .into()
}

/// The panel: a heading, one row per account, and the row that adds a Microsoft account.
fn panel<M: Clone + 'static>(entries: Vec<Entry>, to: fn(Message) -> M) -> Element<'static, M> {
    let rows = entries
        .into_iter()
        .enumerate()
        .map(|(index, entry)| account_row(index, entry, to))
        .collect::<Vec<_>>();
    let add = TextButton::labeled_icon(Icon::lucide().plus(), lang::t("settings.accounts.add_microsoft"), Variant::Ghost)
        .width(Fill)
        .on_press(to(Message::Add));
    container(
        column![
            text(lang::t("nav.accounts"))
                .size(12)
                .font(fonts::Inter::SEMI_BOLD)
                .color(color::text_muted()),
            column(rows).spacing(4),
            container(Space::new().height(1)).width(Fill).style(|_| container::Style {
                background: Some(Background::Color(color::surface_line())),
                ..container::Style::default()
            }),
            add,
        ]
        .spacing(8)
        .width(Fill),
    )
    .width(PANEL_WIDTH)
    .padding(Padding::new(10.0))
    .style(|_| container::Style {
        background: Some(Background::Color(color::surface_bg3())),
        border: Border {
            radius: 14.0.into(),
            width: 1.0,
            color: color::surface_line_strong(),
        },
        ..container::Style::default()
    })
    .into()
}

/// One account: its avatar, name and kind, and a check mark when it plays. A press on it switches to it.
fn account_row<M: Clone + 'static>(index: usize, entry: Entry, to: fn(Message) -> M) -> Element<'static, M> {
    let kind = if entry.microsoft {
        lang::t("settings.accounts.microsoft_account")
    } else {
        lang::t("settings.accounts.offline_profile")
    };
    let playing = entry.playing;
    let details = column![
        text(entry.username.clone())
            .size(14)
            .font(fonts::Inter::SEMI_BOLD)
            .color(color::text_headline()),
        text(kind).size(12).font(fonts::Inter::REGULAR).color(color::text_muted()),
    ]
    .spacing(2);
    let mut content = row![avatar(index, &entry.username, entry.face), details, Space::new().width(Fill)]
        .spacing(12)
        .align_y(alignment::Vertical::Center);
    if playing {
        content = content.push(utility::icon(Icon::lucide().check(), 14.0, Some((color::primary(), color::primary()))));
    }
    button(content)
        .width(Fill)
        .padding(Padding::new(8.0))
        .on_press(to(Message::Switch(index)))
        .style(move |_, status| button::Style {
            background: Some(Background::Color(row_fill(playing, status))),
            border: Border {
                radius: 10.0.into(),
                ..Border::default()
            },
            text_color: color::text_headline(),
            ..button::Style::default()
        })
        .into()
}

/// The fill of a row: the tint under the account that plays, a faint white under the pointer, and none otherwise.
fn row_fill(playing: bool, status: button::Status) -> Color {
    match (playing, status) {
        (true, _) => color::primary_tint(),
        (false, button::Status::Hovered | button::Status::Pressed) => rgb!(255, 255, 255, 6%),
        _ => Color::TRANSPARENT,
    }
}

/// The account's face when it has one, otherwise its initial on a colored tile.
fn avatar<'a, M: 'a>(index: usize, name: &str, face: Option<Handle>) -> Element<'a, M> {
    if let Some(handle) = face {
        return image(handle).width(ROW_AVATAR_SIZE).height(ROW_AVATAR_SIZE).into();
    }
    let initial = name.chars().next().map(|letter| letter.to_uppercase().to_string()).unwrap_or_default();
    let fill = AVATAR_ACCENTS[index % AVATAR_ACCENTS.len()].color();
    container(text(initial).size(14).font(fonts::Inter::SEMI_BOLD).color(color::surface_bg0()))
        .width(ROW_AVATAR_SIZE)
        .height(ROW_AVATAR_SIZE)
        .align_x(alignment::Horizontal::Center)
        .align_y(alignment::Vertical::Center)
        .style(move |_| container::Style {
            background: Some(Background::Color(fill)),
            border: Border {
                radius: 10.0.into(),
                ..Border::default()
            },
            ..container::Style::default()
        })
        .into()
}
