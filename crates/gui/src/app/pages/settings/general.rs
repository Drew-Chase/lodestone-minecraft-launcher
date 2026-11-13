//! The General tab: where the launcher starts and where instances live, how it updates itself, and the window every
//! instance opens with.

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
use crate::app::paths::{self, LauncherPaths};
use crate::app::preferences::{HERO_RULES, HeroRule, SettingsValues, StartupPage};
use crate::app::theme::{color, fonts};

/// The pages the launcher can open on, in the order the select offers them.
const STARTUP_PAGES: [StartupPage; 4] = [StartupPage::Library, StartupPage::News, StartupPage::Discover, StartupPage::Worlds];

/// The width of the selects on this tab.
const SELECT_WIDTH: f32 = 240.0;

/// The width the data folder's path wraps within.
const FOLDER_PATH_WIDTH: f32 = 300.0;

/// The General tab: the startup and folder rows, the update switches and the window.
pub fn view<'a>(
    shown: Shown<'a>,
    resolution: &'a ResolutionDraft,
    settings: &'a SettingsValues,
    paths: &'a LauncherPaths,
    can_update: bool,
) -> Element<'a, Message> {
    let startup = vec![
        folder_row(paths),
        startup_row(settings.startup_page),
        hero_row(settings.hero),
        game_start_row(shown.values.hide_launcher),
        game_exit_row(shown.values.quit_after_exit),
    ];
    column![
        plain_card(divided(startup)),
        card(
            lang::t("settings.general.updates"),
            lang::t("settings.general.updates_subtitle"),
            divided(update_rows(settings, can_update)),
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

/// The folder the launcher keeps its data in, shown with the home folder as `~`, with buttons that open it and move it. A
/// portable launcher's folder and a development run's are fixed, so they have no move buttons.
fn folder_row<'a>(paths: &LauncherPaths) -> Element<'a, Message> {
    let root = paths.root();
    // The path wraps within a fixed width, and the buttons sit under it, so a long path never squeezes the row's label.
    let path = text(display_path(&root))
        .size(12)
        .font(fonts::JetBrainsMono::REGULAR)
        .color(color::text_muted())
        .width(FOLDER_PATH_WIDTH)
        .align_x(alignment::Horizontal::Right);
    let mut buttons = row![TextButton::ghost(lang::t("settings.general.open_folder")).on_press(Message::OpenDataFolder)]
        .spacing(12)
        .align_y(alignment::Vertical::Center);
    if !paths::data_folder_fixed() {
        buttons = buttons.push(TextButton::ghost(lang::t("settings.general.change_folder")).on_press(Message::ChooseDataFolder));
        if root != paths::default_data_root() {
            buttons = buttons.push(TextButton::ghost(lang::t("settings.general.default_folder")).on_press(Message::UseDefaultDataFolder));
        }
    }
    let control = column![path, buttons].spacing(10).align_x(alignment::Horizontal::Right);
    described_setting(lang::t("settings.general.folder"), lang::t("settings.general.folder_hint"), control)
}

/// The page the launcher opens on.
fn startup_row<'a>(page: StartupPage) -> Element<'a, Message> {
    let select = Select::new(
        [
            lang::t("settings.general.startup_library"),
            lang::t("settings.general.startup_news"),
            lang::t("settings.general.startup_discover"),
            lang::t("settings.general.startup_worlds"),
        ],
        STARTUP_PAGES.iter().position(|option| *option == page),
    )
    .on_select(|index| Message::Edit(Edit::Startup(STARTUP_PAGES[index])))
    .width(SELECT_WIDTH);
    described_setting(lang::t("settings.general.startup"), lang::t("settings.general.startup_hint"), select)
}

/// Which instance the library's banner features. Each rule picks one instance, and the select lists the rules in the order
/// `HERO_RULES` gives them.
fn hero_row<'a>(rule: HeroRule) -> Element<'a, Message> {
    let select = Select::new(
        [
            lang::t("settings.general.hero_last_played"),
            lang::t("settings.general.hero_last_installed"),
            lang::t("settings.general.hero_last_both"),
            lang::t("settings.general.hero_most_played"),
        ],
        HERO_RULES.iter().position(|option| *option == rule),
    )
    .on_select(|index| Message::Edit(Edit::Hero(HERO_RULES[index])))
    .width(SELECT_WIDTH);
    described_setting(lang::t("settings.general.hero"), lang::t("settings.general.hero_hint"), select)
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

/// The update switches. Auto-update Lodestone appears only where this copy can install an update itself. Anywhere else the
/// switch would do nothing, so it is not shown.
fn update_rows<'a>(settings: &SettingsValues, can_update: bool) -> Vec<Element<'a, Message>> {
    let auto_update = can_update.then(|| {
        described_setting(
            lang::t("settings.general.auto_update"),
            lang::t("settings.general.auto_update_hint"),
            Toggle::new(settings.auto_update).on_toggle(Message::Edit(Edit::AutoUpdate(!settings.auto_update))),
        )
    });
    auto_update
        .into_iter()
        .chain([
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
        ])
        .collect()
}
