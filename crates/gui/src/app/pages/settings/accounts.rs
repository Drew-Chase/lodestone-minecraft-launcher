//! The Accounts tab: the accounts the launcher keeps signed in, and the skin browser. Making one the default, signing one
//! out, and adding a Microsoft account each act at once, because each is a sign-in the root carries out. The skin the
//! browser stages is uploaded on Save, like the rest of the page.

pub mod skins;
pub mod viewer;

use std::sync::Arc;

use aurora_leaf::Icon;
use iced::widget::image::FilterMethod;
use iced::widget::{Row, Space, button, column, container, image, row, text};
use iced::{Background, Border, Element, Fill, Padding, alignment};

use self::skins::LibrarySkin;
use self::viewer::{Layers, Model, Pose, Skin};
use super::{Message, card};
use crate::app::components::button::Variant;
use crate::app::components::menu::MenuItem;
use crate::app::components::{Dropdown, TextButton};
use crate::app::lang;
use crate::app::storage::{StoredAccount, StoredCredentials};
use crate::app::theme::{Accent, color, fonts};

/// The side of an account's avatar, in pixels.
const AVATAR_SIZE: f32 = 44.0;

/// The colors an account without a face takes its initial on, cycled by its place in the list.
const AVATAR_ACCENTS: [Accent; 5] = [Accent::Green, Accent::Violet, Accent::Cyan, Accent::Amber, Accent::Pink];

/// The side of a saved skin's thumbnail, in pixels.
const THUMBNAIL_SIZE: f32 = 56.0;

/// How many thumbnails share a row of the grid.
const THUMBNAILS_PER_ROW: usize = 6;

/// The width of the panel that shows the selected skin.
const INFO_WIDTH: f32 = 240.0;

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

/// One of the overlay layers the viewer draws.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Layer {
    Hat,
    Jacket,
    Sleeves,
    Pants,
}

/// A change to the skin browser. The page applies each one, except opening the folder, which the root reveals.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum SkinAction {
    /// Shows the saved skin at this index in the viewer.
    View(usize),
    /// Stages the saved skin at this index, to upload on Save.
    Stage(usize),
    /// Shows the account's own skin in the viewer again.
    ShowCurrent,
    /// Changes the pose.
    Pose(Pose),
    /// Shows or hides an overlay layer.
    Layer(Layer),
    /// Chooses the classic or the slim model.
    Slim(bool),
    /// Opens the folder the saved skins live in.
    OpenFolder,
}

/// What the skin browser shows: the saved skins, what the viewer shows, and the controls' state.
pub struct BrowserView<'a> {
    pub library: &'a [LibrarySkin],
    /// The saved skin the viewer shows, when one is chosen.
    pub viewing: Option<usize>,
    /// The saved skin staged for upload on Save.
    pub staged: Option<usize>,
    /// The decoded saved skin the viewer shows.
    pub preview: Option<&'a Arc<Skin>>,
    /// The skin the account uses, decoded.
    pub current: Option<&'a Arc<Skin>>,
    pub pose: Pose,
    pub layers: Layers,
    pub slim: bool,
}

/// The Accounts tab: the signed-in list, and the skin browser under it. `face` is the face of the account that plays.
pub fn view<'a>(credentials: &'a StoredCredentials, face: Option<&'a image::Handle>, browser: BrowserView<'a>) -> Element<'a, Message> {
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
    column![
        card(
            lang::t("settings.accounts.signed_in_title"),
            lang::t("settings.accounts.signed_in_subtitle"),
            column![column(rows.collect::<Vec<_>>()).spacing(10), add].spacing(14).width(Fill).into(),
        ),
        skin_browser(browser),
    ]
    .spacing(24)
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

/// The skin browser: the viewer with its layer and pose controls, the selected skin's details, and the saved skins.
fn skin_browser<'a>(browser: BrowserView<'a>) -> Element<'a, Message> {
    let count = lang::t_count("settings.skins.count", browser.library.len());
    let open = TextButton::ghost(lang::t("settings.skins.open_folder")).on_press(Message::Skin(SkinAction::OpenFolder));
    let shown = browser.preview.or(browser.current).cloned();
    let model = Model {
        skin: shown,
        pose: browser.pose,
        layers: browser.layers,
        slim: browser.slim,
    };
    let stage = column![
        row![
            layer_column(browser.layers),
            container(viewer::viewer(model)).width(Fill),
            pose_column(browser.pose),
        ]
        .spacing(12)
        .align_y(alignment::Vertical::Center),
    ]
    .width(Fill);
    let body = row![stage, info_panel(&browser)].spacing(20).width(Fill);
    card(
        lang::t("settings.skins.title"),
        lang::t("settings.skins.subtitle"),
        column![
            row![
                text(count).size(13).font(fonts::Inter::REGULAR).color(color::text_muted()),
                Space::new().width(Fill),
                open
            ]
            .align_y(alignment::Vertical::Center),
            body,
            thumbnails(&browser),
        ]
        .spacing(18)
        .width(Fill)
        .into(),
    )
}

/// The overlay layer switches, beside the viewer.
fn layer_column<'a>(layers: Layers) -> Element<'a, Message> {
    let switches = [
        (Layer::Hat, layers.hat, lang::t("settings.skins.layer_hat")),
        (Layer::Jacket, layers.jacket, lang::t("settings.skins.layer_jacket")),
        (Layer::Sleeves, layers.sleeves, lang::t("settings.skins.layer_sleeves")),
        (Layer::Pants, layers.pants, lang::t("settings.skins.layer_pants")),
    ];
    column(switches.map(|(layer, on, label)| choice_button(label, on, Message::Skin(SkinAction::Layer(layer)))))
        .spacing(8)
        .width(96)
        .into()
}

/// The pose choices, beside the viewer.
fn pose_column<'a>(pose: Pose) -> Element<'a, Message> {
    let labels = [
        (Pose::Stand, lang::t("settings.skins.pose_stand")),
        (Pose::Arms, lang::t("settings.skins.pose_arms")),
        (Pose::Walk, lang::t("settings.skins.pose_walk")),
        (Pose::Run, lang::t("settings.skins.pose_run")),
        (Pose::Crouch, lang::t("settings.skins.pose_crouch")),
    ];
    column(labels.map(|(choice, label)| choice_button(label, choice == pose, Message::Skin(SkinAction::Pose(choice)))))
        .spacing(8)
        .width(96)
        .into()
}

/// A small switch that shows whether it is on, and sends `on_press` when pressed.
fn choice_button<'a>(label: &'static str, on: bool, on_press: Message) -> Element<'a, Message> {
    button(
        container(
            text(label)
                .size(12)
                .font(fonts::Inter::MEDIUM)
                .color(if on { color::primary() } else { color::text_muted() }),
        )
        .width(Fill)
        .padding(Padding::new(0.0).top(7.0).bottom(7.0).left(10.0).right(10.0)),
    )
    .width(Fill)
    .padding(0)
    .on_press(on_press)
    .style(move |_, _| button::Style {
        background: Some(Background::Color(if on { color::primary_tint() } else { color::surface_bg3() })),
        border: Border {
            radius: 10.0.into(),
            width: 1.0,
            color: if on { color::primary_glow() } else { color::surface_line() },
        },
        ..button::Style::default()
    })
    .into()
}

/// The panel beside the viewer: the skin in use, its model, size and color, and the stage button for a saved skin.
fn info_panel<'a>(browser: &BrowserView<'a>) -> Element<'a, Message> {
    let saved = browser.viewing.and_then(|index| browser.library.get(index));
    let name = match saved {
        Some(skin) => skin.name.clone(),
        None => lang::t("settings.skins.current_name").to_owned(),
    };
    let size = match saved {
        Some(skin) => skins::dimensions(&skin.png).map_or_else(String::new, |(width, height)| format!("{width}×{height}")),
        None => "64×64".to_owned(),
    };
    let accent = saved.and_then(|skin| skins::accent_hex(&skin.png)).unwrap_or_else(|| "—".to_owned());
    let mut panel = column![
        text(lang::t("settings.skins.selected"))
            .size(10)
            .font(fonts::JetBrainsMono::SEMI_BOLD)
            .color(color::text_muted()),
        text(name).size(18).font(fonts::Inter::SEMI_BOLD).color(color::text_headline()),
        crate::app::components::segmented::segmented(
            &[(false, lang::t("settings.skins.classic")), (true, lang::t("settings.skins.slim")),],
            browser.slim,
            |slim| Message::Skin(SkinAction::Slim(slim)),
        ),
        detail_line(lang::t("settings.skins.resolution"), size),
        detail_line(lang::t("settings.skins.accent"), accent),
    ]
    .spacing(12)
    .width(INFO_WIDTH);
    if let Some(index) = browser.viewing {
        panel = if browser.staged == Some(index) {
            panel.push(
                text(lang::t("settings.skins.staged"))
                    .size(12)
                    .font(fonts::Inter::REGULAR)
                    .color(color::primary()),
            )
        } else {
            panel.push(TextButton::primary(lang::t("settings.skins.stage")).on_press(Message::Skin(SkinAction::Stage(index))))
        };
        panel = panel.push(TextButton::ghost(lang::t("settings.skins.show_current")).on_press(Message::Skin(SkinAction::ShowCurrent)));
    }
    panel.into()
}

/// One labelled value in the selected panel.
fn detail_line<'a>(label: &'static str, value: String) -> Element<'a, Message> {
    row![
        text(label).size(11).font(fonts::JetBrainsMono::REGULAR).color(color::text_muted()),
        Space::new().width(Fill),
        text(value).size(12).font(fonts::JetBrainsMono::SEMI_BOLD).color(color::text_headline()),
    ]
    .align_y(alignment::Vertical::Center)
    .into()
}

/// The saved skins as thumbnails, a row of them at a time. The viewed skin and the staged one are outlined.
fn thumbnails<'a>(browser: &BrowserView<'a>) -> Element<'a, Message> {
    if browser.library.is_empty() {
        return text(lang::t("settings.skins.empty"))
            .size(12)
            .font(fonts::Inter::REGULAR)
            .color(color::text_muted())
            .into();
    }
    let tiles: Vec<Element<'a, Message>> = browser
        .library
        .iter()
        .enumerate()
        .map(|(index, skin)| thumbnail(index, skin, browser.viewing == Some(index), browser.staged == Some(index)))
        .collect();
    let mut rows = column![].spacing(10);
    let mut current = Row::new().spacing(10);
    for (count, tile) in tiles.into_iter().enumerate() {
        current = current.push(tile);
        if (count + 1) % THUMBNAILS_PER_ROW == 0 {
            rows = rows.push(std::mem::replace(&mut current, Row::new().spacing(10)));
        }
    }
    if !browser.library.len().is_multiple_of(THUMBNAILS_PER_ROW) {
        rows = rows.push(current);
    }
    rows.into()
}

/// One saved skin: its picture and name. Pressing it views the skin.
fn thumbnail<'a>(index: usize, skin: &LibrarySkin, viewing: bool, staged: bool) -> Element<'a, Message> {
    let outline = if staged {
        color::primary()
    } else if viewing {
        color::text_headline()
    } else {
        color::surface_line()
    };
    let picture = image(skin.handle.clone())
        .width(THUMBNAIL_SIZE)
        .height(THUMBNAIL_SIZE)
        .filter_method(FilterMethod::Nearest);
    button(
        column![
            picture,
            text(skin.name.clone()).size(11).font(fonts::Inter::REGULAR).color(color::text_muted()),
        ]
        .spacing(6)
        .align_x(alignment::Horizontal::Center),
    )
    .padding(8)
    .on_press(Message::Skin(SkinAction::View(index)))
    .style(move |_, _| button::Style {
        background: Some(Background::Color(color::surface_bg3())),
        border: Border {
            radius: 12.0.into(),
            width: 1.0,
            color: outline,
        },
        ..button::Style::default()
    })
    .into()
}
