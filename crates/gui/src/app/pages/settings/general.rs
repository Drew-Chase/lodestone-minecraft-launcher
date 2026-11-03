//! The General tab: where the launcher starts and where instances live, how it updates itself, how many files an install
//! fetches at once, and the window every instance opens with.

use std::path::Path;

use iced::widget::{column, row, text};
use iced::{Element, Fill, alignment};

use super::edit::Edit;
use super::{Message, card, display_path, plain_card};
use crate::app::components::launch_rows::{self, ResolutionDraft, Shown};
use crate::app::components::panel::divided;
use crate::app::components::setting_row::described_setting;
use crate::app::components::{Select, TextButton, Toggle};
use crate::app::lang;
use crate::app::launch_settings::LaunchChange;
use crate::app::preferences::{CONCURRENT_DOWNLOADS, SettingsValues, StartupPage};
use crate::app::theme::{color, fonts};

/// The pages the launcher can open on, in the order the select offers them.
const STARTUP_PAGES: [StartupPage; 3] = [StartupPage::Library, StartupPage::Discover, StartupPage::Worlds];

/// The width of the selects on this tab.
const SELECT_WIDTH: f32 = 240.0;

/// The General tab: the startup and folder rows, the update switches, the download count and the window.
pub fn view<'a>(shown: Shown<'a>, resolution: &'a ResolutionDraft, settings: &'a SettingsValues, instances: &'a Path) -> Element<'a, Message> {
    let startup = vec![
        folder_row(instances),
        startup_row(settings.startup_page),
        game_start_row(shown.values.hide_launcher),
        game_exit_row(shown.values.quit_after_exit),
    ];
    column![
        plain_card(divided(startup)),
        card(
            lang::t("settings.general.updates"),
            lang::t("settings.general.updates_subtitle"),
            divided(update_rows(settings)),
        ),
        card(
            lang::t("settings.general.downloads"),
            lang::t("settings.general.downloads_subtitle"),
            downloads(settings.concurrent_downloads),
        ),
        card(
            lang::t("instance.settings.section.window"),
            lang::t("settings.general.window_subtitle"),
            divided(launch_rows::display_rows(shown, resolution, Message::Change, Message::Resolution)),
        ),
    ]
    .spacing(24)
    .width(Fill)
    .into()
}

/// The folder instances live in, shown with the home folder as `~`, and a button that opens it.
fn folder_row<'a>(instances: &Path) -> Element<'a, Message> {
    let path = text(display_path(instances))
        .size(12)
        .font(fonts::JetBrainsMono::REGULAR)
        .color(color::text_muted())
        .width(Fill);
    let control = row![
        path,
        TextButton::ghost(lang::t("settings.general.open_folder")).on_press(Message::OpenInstancesFolder),
    ]
    .spacing(12)
    .align_y(alignment::Vertical::Center);
    described_setting(lang::t("settings.general.folder"), lang::t("settings.general.folder_hint"), control)
}

/// The page the launcher opens on.
fn startup_row<'a>(page: StartupPage) -> Element<'a, Message> {
    let select = Select::new(
        [
            lang::t("settings.general.startup_library"),
            lang::t("settings.general.startup_discover"),
            lang::t("settings.general.startup_worlds"),
        ],
        STARTUP_PAGES.iter().position(|option| *option == page),
    )
    .on_select(|index| Message::Edit(Edit::Startup(STARTUP_PAGES[index])))
    .width(SELECT_WIDTH);
    described_setting(lang::t("settings.general.startup"), lang::t("settings.general.startup_hint"), select)
}

/// Whether the launcher minimizes while a game runs. The launch default is `hide_launcher`, which an instance inherits.
fn game_start_row<'a>(hide: bool) -> Element<'a, Message> {
    let select = Select::new(
        [lang::t("settings.general.on_start_minimize"), lang::t("settings.general.on_start_keep")],
        Some(usize::from(!hide)),
    )
    .on_select(|index| Message::Change(LaunchChange::HideLauncher(index == 0)))
    .width(SELECT_WIDTH);
    described_setting(lang::t("settings.general.on_start"), lang::t("settings.general.on_start_hint"), select)
}

/// Whether the launcher quits once a game exits. The launch default is `quit_after_exit`, which an instance inherits.
fn game_exit_row<'a>(quit: bool) -> Element<'a, Message> {
    let select = Select::new(
        [lang::t("settings.general.on_exit_restore"), lang::t("settings.general.on_exit_quit")],
        Some(usize::from(quit)),
    )
    .on_select(|index| Message::Change(LaunchChange::QuitAfterExit(index == 1)))
    .width(SELECT_WIDTH);
    described_setting(lang::t("settings.general.on_exit"), lang::t("settings.general.on_exit_hint"), select)
}

/// The three update switches.
fn update_rows<'a>(settings: &SettingsValues) -> Vec<Element<'a, Message>> {
    vec![
        described_setting(
            lang::t("settings.general.auto_update"),
            lang::t("settings.general.auto_update_hint"),
            Toggle::new(settings.auto_update).on_toggle(Message::Edit(Edit::AutoUpdate(!settings.auto_update))),
        ),
        described_setting(
            lang::t("settings.general.beta"),
            lang::t("settings.general.beta_hint"),
            Toggle::new(settings.beta_channel).on_toggle(Message::Edit(Edit::BetaChannel(!settings.beta_channel))),
        ),
        described_setting(
            lang::t("settings.general.auto_update_versions"),
            lang::t("settings.general.auto_update_versions_hint"),
            Toggle::new(settings.auto_update_game_versions)
                .on_toggle(Message::Edit(Edit::AutoUpdateGameVersions(!settings.auto_update_game_versions))),
        ),
    ]
}

/// The slider for how many files an install fetches at once, with its value.
fn downloads<'a>(count: u16) -> Element<'a, Message> {
    let (low, high) = (*CONCURRENT_DOWNLOADS.start(), *CONCURRENT_DOWNLOADS.end());
    let bar = iced::widget::slider(f32::from(low)..=f32::from(high), f32::from(count), |value| {
        Message::Edit(Edit::ConcurrentDownloads(value.round() as u16))
    })
    .step(1.0_f32)
    .style(crate::app::components::slider::style)
    .width(Fill);
    row![
        bar,
        text(count.to_string())
            .size(14)
            .font(fonts::JetBrainsMono::SEMI_BOLD)
            .color(color::primary()),
    ]
    .spacing(16)
    .align_y(alignment::Vertical::Center)
    .into()
}
