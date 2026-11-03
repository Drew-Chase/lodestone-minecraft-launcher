//! The Accounts tab: the accounts the launcher keeps signed in. Making one the default, signing one out, and adding a
//! Microsoft account each act at once, because each is a sign-in the root carries out. The skin and the cape an account
//! wears are on the Skins page.

use aurora_leaf::Icon;
use iced::widget::{Space, column, container, image, row, text};
use iced::{Background, Border, Element, Fill, Padding, alignment};

use super::{Message, card};
use crate::app::components::button::Variant;
use crate::app::components::menu::MenuItem;
use crate::app::components::{Dropdown, TextButton};
use crate::app::lang;
use crate::app::pages::Page;
use crate::app::storage::{StoredAccount, StoredCredentials};
use crate::app::theme::{Accent, color, fonts};

/// The side of an account's avatar, in pixels.
const AVATAR_SIZE: f32 = 44.0;

/// The colors an account without a face takes its initial on, cycled by its place in the list.
const AVATAR_ACCENTS: [Accent; 5] = [Accent::Green, Accent::Violet, Accent::Cyan, Accent::Amber, Accent::Pink];

/// A change to the signed-in accounts. The root carries each one out, since each is a sign-in.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum AccountAction {
    /// Makes the account at this index the one that plays.
    MakePrimary(usize),
    /// Signs the account at this index out.
    SignOut(usize),
    /// Signs a Microsoft account in, and makes it the one that plays.
    Add,
}

/// The Accounts tab: the signed-in list, and a way to the Skins page. `face` is the face of the account that plays.
pub fn view<'a>(credentials: &'a StoredCredentials, face: Option<&'a image::Handle>) -> Element<'a, Message> {
    let playing = credentials.primary_index();
    let rows = credentials.accounts.iter().enumerate().map(|(index, account)| {
        let is_playing = playing == Some(index);
        let face = if is_playing && matches!(account, StoredAccount::Microsoft(_)) {
            face
        } else {
            None
        };
        account_row(index, account, is_playing, face)
    });
    let add = TextButton::labeled_icon(Icon::lucide().plus(), lang::t("settings.accounts.add_microsoft"), Variant::Ghost)
        .width(Fill)
        .on_press(Message::Account(AccountAction::Add));
    let skins = row![
        text(lang::t("settings.accounts.skins_hint"))
            .size(13)
            .font(fonts::Inter::REGULAR)
            .color(color::text_muted()),
        Space::new().width(Fill),
        TextButton::ghost(lang::t("settings.accounts.open_skins")).on_press(Message::Navigate(Page::Skins)),
    ]
    .align_y(alignment::Vertical::Center);
    column![card(
        lang::t("settings.accounts.signed_in_title"),
        lang::t("settings.accounts.signed_in_subtitle"),
        column![column(rows.collect::<Vec<_>>()).spacing(10), add, skins]
            .spacing(14)
            .width(Fill)
            .into(),
    )]
    .width(Fill)
    .into()
}

/// One account: its avatar, name and kind, a PRIMARY tag when it plays, and a menu with the actions on it.
fn account_row<'a>(index: usize, account: &StoredAccount, playing: bool, face: Option<&'a image::Handle>) -> Element<'a, Message> {
    let kind = match account {
        StoredAccount::Microsoft(_) => lang::t("settings.accounts.microsoft_account"),
        StoredAccount::Offline { .. } => lang::t("settings.accounts.offline_profile"),
    };
    let mut title = row![
        text(account.username().to_owned())
            .size(15)
            .font(fonts::Inter::SEMI_BOLD)
            .color(color::text_headline()),
    ]
    .spacing(10)
    .align_y(alignment::Vertical::Center);
    if playing {
        title = title.push(primary_tag());
    }
    let details = column![title, text(kind).size(12).font(fonts::Inter::REGULAR).color(color::text_muted()),].spacing(4);
    let menu = Dropdown::new(Icon::lucide().ellipsis_vertical())
        .items([
            MenuItem::Action {
                label: lang::t("settings.accounts.make_primary").to_owned(),
                icon: None,
                danger: false,
            },
            MenuItem::Action {
                label: lang::t("settings.accounts.sign_out").to_owned(),
                icon: None,
                danger: true,
            },
        ])
        .on_select(move |choice| {
            Message::Account(if choice == 0 {
                AccountAction::MakePrimary(index)
            } else {
                AccountAction::SignOut(index)
            })
        })
        .width(36)
        .height(36);
    container(
        row![avatar(index, account.username(), face), details, Space::new().width(Fill), menu]
            .spacing(16)
            .align_y(alignment::Vertical::Center),
    )
    .width(Fill)
    .padding(Padding::new(14.0))
    .style(move |_| container::Style {
        background: Some(Background::Color(color::surface_bg3())),
        border: Border {
            radius: 14.0.into(),
            width: 1.0,
            color: if playing { color::primary() } else { color::surface_line() },
        },
        ..container::Style::default()
    })
    .into()
}

/// The account's face when it has one, otherwise its initial on a colored tile.
fn avatar<'a>(index: usize, name: &str, face: Option<&'a image::Handle>) -> Element<'a, Message> {
    if let Some(handle) = face {
        return image(handle.clone()).width(AVATAR_SIZE).height(AVATAR_SIZE).into();
    }
    let initial = name.chars().next().map(|letter| letter.to_uppercase().to_string()).unwrap_or_default();
    let fill = AVATAR_ACCENTS[index % AVATAR_ACCENTS.len()].color();
    container(text(initial).size(16).font(fonts::Inter::SEMI_BOLD).color(color::surface_bg0()))
        .width(AVATAR_SIZE)
        .height(AVATAR_SIZE)
        .align_x(alignment::Horizontal::Center)
        .align_y(alignment::Vertical::Center)
        .style(move |_| container::Style {
            background: Some(Background::Color(fill)),
            border: Border {
                radius: 12.0.into(),
                ..Border::default()
            },
            ..container::Style::default()
        })
        .into()
}

/// The tag that marks the account that plays.
fn primary_tag<'a>() -> Element<'a, Message> {
    container(
        text(lang::t("settings.accounts.primary"))
            .size(10)
            .font(fonts::JetBrainsMono::SEMI_BOLD)
            .color(color::primary()),
    )
    .padding(Padding::new(0.0).left(8.0).right(8.0).top(3.0).bottom(3.0))
    .style(|_| container::Style {
        background: Some(Background::Color(color::primary_tint())),
        border: Border {
            radius: 6.0.into(),
            ..Border::default()
        },
        ..container::Style::default()
    })
    .into()
}
