//! The launcher's folder rows: each folder with what it holds, the path it is in, and the buttons that open it, change it or send
//! it back to its default. The Settings page and the setup wizard list them the same way.

use iced::widget::{Text, column, row};
use iced::{Element, Fill, Padding, alignment};

use crate::app::components::TextButton;
use crate::app::data_folder::{FolderAction, FolderKind, Row};
use crate::app::lang;
use crate::app::pages::settings::display_path;
use crate::app::theme::{color, fonts};

/// One row per folder: its name, what it holds and its path on the left, and its buttons on the right. `act` builds the message a
/// button sends for the folder it is on and the action it asks for.
pub fn rows<M: Clone + 'static>(folders: &[Row], act: impl Fn(FolderKind, FolderAction) -> M + Copy + 'static) -> Vec<Element<'static, M>> {
    folders.iter().map(|folder| folder_row(folder, act)).collect()
}

/// The row of one folder: its name, what it holds and its path, and its buttons.
fn folder_row<M: Clone + 'static>(folder: &Row, act: impl Fn(FolderKind, FolderAction) -> M + Copy + 'static) -> Element<'static, M> {
    let kind = folder.kind;
    let details = column![
        Text::new(lang::t(name_key(kind)))
            .size(14)
            .font(fonts::Inter::SEMI_BOLD)
            .color(color::text_headline()),
        Text::new(lang::t(hint_key(kind)))
            .size(11)
            .font(fonts::Inter::REGULAR)
            .color(color::text_muted()),
        Text::new(display_path(&folder.path))
            .size(11)
            .font(fonts::JetBrainsMono::REGULAR)
            .color(color::text_muted()),
    ]
    .spacing(4)
    .width(Fill);
    let mut buttons = row![TextButton::ghost(lang::t("folders.open")).on_press(act(kind, FolderAction::Open))]
        .spacing(12)
        .align_y(alignment::Vertical::Center);
    if folder.editable {
        buttons = buttons.push(TextButton::ghost(lang::t("folders.change")).on_press(act(kind, FolderAction::Change)));
        if !folder.default {
            buttons = buttons.push(TextButton::ghost(lang::t("folders.default")).on_press(act(kind, FolderAction::Reset)));
        }
    }
    // The padding matches the other settings rows, so the folders sit evenly among them.
    row![details, buttons]
        .spacing(16)
        .align_y(alignment::Vertical::Center)
        .padding(Padding::new(0.0).top(14).bottom(14))
        .width(Fill)
        .into()
}

/// The name of a folder's row.
fn name_key(kind: FolderKind) -> &'static str {
    match kind {
        FolderKind::Instances => "folders.instances.name",
        FolderKind::Shared => "folders.shared.name",
        FolderKind::Logs => "folders.logs.name",
        FolderKind::Cache => "folders.cache.name",
        FolderKind::Config => "folders.config.name",
    }
}

/// What a folder holds, and when a change to it takes effect.
fn hint_key(kind: FolderKind) -> &'static str {
    match kind {
        FolderKind::Instances => "folders.instances.hint",
        FolderKind::Shared => "folders.shared.hint",
        FolderKind::Logs => "folders.logs.hint",
        FolderKind::Cache => "folders.cache.hint",
        FolderKind::Config => "folders.config.hint",
    }
}
