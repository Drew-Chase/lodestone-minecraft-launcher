//! The Done step: a summary of what the setup did, and the note on what changes at the next start.

use iced::widget::{Text, column, row};
use iced::{Element, Fill, Length, alignment};

use super::Message;
use crate::app::lang;
use crate::app::pages::settings::display_path;
use crate::app::paths::LauncherPaths;
use crate::app::theme::{color as palette, fonts};

/// The summary: the account, the folders, what the import and Discover brought in, and the note on the folders that take effect
/// at the next start.
pub fn view<'a>(account: Option<&'a str>, paths: &LauncherPaths, imported: usize, queued: usize) -> Element<'a, Message> {
    let account_line = match account {
        Some(name) => lang::t_with("setup.done.signed_in", &[("name", name)]).to_string(),
        None => lang::t("setup.done.no_account").to_string(),
    };
    let mut lines = column![
        line(lang::t("setup.done.account"), account_line),
        line(lang::t("folders.instances.name"), display_path(&paths.instances())),
        line(lang::t("folders.shared.name"), display_path(&paths.root())),
        line(lang::t("setup.done.imported"), imported.to_string()),
        line(lang::t("setup.done.queued"), queued.to_string()),
    ]
    .spacing(14)
    .width(Fill);
    if paths.logs_chosen().is_some() || paths.cache_chosen().is_some() {
        lines = lines.push(
            Text::new(lang::t("folders.restart_note"))
                .size(12)
                .font(fonts::Inter::REGULAR)
                .color(palette::text_muted()),
        );
    }
    lines.into()
}

/// One line of the summary: its label, and what it says.
fn line<'a>(label: &'static str, value: String) -> Element<'a, Message> {
    row![
        Text::new(label)
            .size(12)
            .font(fonts::Inter::SEMI_BOLD)
            .color(palette::text_muted())
            .width(Length::Fixed(170.0)),
        Text::new(value)
            .size(13)
            .font(fonts::Inter::MEDIUM)
            .color(palette::text_headline())
            .width(Fill),
    ]
    .spacing(16)
    .align_y(alignment::Vertical::Center)
    .into()
}
