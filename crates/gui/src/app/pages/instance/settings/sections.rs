//! The Game Version, Paths & Files and Danger Zone sections. The launch sections are built from the shared rows in
//! `components::launch_rows`, and the General section in `general.rs`.

use std::path::Path;

use iced::widget::{Space, Text, column, row};
use iced::{Element, Fill, Padding, alignment};
use lodestone_loaders::LoaderVersion;

use super::{Edit, Message};
use crate::app::components::badge::{Badge, loader_palette};
use crate::app::components::button::Variant;
use crate::app::components::launch_rows;
use crate::app::components::panel::{bordered_panel, divided, divided_panel};
use crate::app::components::setting_row::{self, described_setting, setting};
use crate::app::components::{Select, TextButton, Toggle, anim};
use crate::app::install::catalog::VersionEntry;
use crate::app::lang;
use crate::app::pages::library::instance::{Instance, Loader};
use crate::app::theme::{color, fonts};

/// The Game Version rows: the Minecraft version and the loader build, the loader's type, and the snapshots toggle.
/// The lists show their own errors, and the loader rows are left out for vanilla.
pub fn game_version<'a>(
    instance: &'a Instance,
    versions: &'a [VersionEntry],
    versions_error: Option<&'a str>,
    builds: Option<&'a [LoaderVersion]>,
    builds_error: Option<&'a str>,
) -> Element<'a, Message> {
    let snapshots = instance.settings.snapshots;
    let ids: Vec<String> = versions
        .iter()
        .filter(|version| snapshots || !version.snapshot)
        .map(|version| version.id.clone())
        .collect();
    let selected = ids.iter().position(|id| *id == instance.mc_version);
    let minecraft = Select::new(ids.clone(), selected)
        .on_select(move |index| Message::Edit(Edit::MinecraftVersion(ids[index].clone())))
        .searchable()
        .width(180);

    let mut rows = vec![setting(lang::t("instance.settings.minecraft"), minecraft)];

    if instance.loader == Loader::Vanilla {
        let (ink, fill) = loader_palette(instance.loader);
        let badge = Badge::new(instance.loader.label(), ink, fill).with_loader(instance.loader);
        rows.push(setting(lang::t("instance.settings.mod_loader"), badge));
    } else {
        rows.push(setting(lang::t("instance.settings.mod_loader"), loader_control(instance, builds)));
    }

    let snapshot_toggle = Toggle::new(snapshots).on_toggle(Message::Edit(Edit::Snapshots(!snapshots)));
    rows.push(setting(lang::t("instance.settings.snapshots"), snapshot_toggle));

    if let Some(error) = versions_error {
        rows.push(error_row(error));
    }
    if let Some(error) = builds_error {
        rows.push(error_row(error));
    }
    divided_panel(rows, launch_rows::panel_padding())
}

/// The loader's type badge beside its build select. The build list is empty while it loads.
fn loader_control<'a>(instance: &'a Instance, builds: Option<&'a [LoaderVersion]>) -> Element<'a, Message> {
    let (ink, fill) = loader_palette(instance.loader);
    let badge = Badge::new(instance.loader.label(), ink, fill).with_loader(instance.loader);
    let versions: Vec<String> = builds.unwrap_or_default().iter().map(|build| build.version.clone()).collect();
    let selected = versions.iter().position(|version| Some(version) == instance.loader_version.as_ref());
    let picks = versions.clone();
    let build = Select::new(versions, selected)
        .placeholder(lang::t("instance.settings.loader_loading"))
        .on_select(move |index| Message::Edit(Edit::LoaderBuild(picks[index].clone())))
        .searchable()
        .width(180);
    row![badge, build].spacing(10).align_y(alignment::Vertical::Center).into()
}

/// A row that names a list's error.
fn error_row<'a>(error: &'a str) -> Element<'a, Message> {
    Text::new(error)
        .size(11)
        .font(fonts::Inter::REGULAR)
        .color(color::buttons::danger_background_hover())
        .into()
}

/// The instance folder, and a button that opens it.
pub fn paths_files(folder: &Path) -> Element<'_, Message> {
    // The path takes the width the button leaves, so a long one wraps instead of pushing the button off the panel.
    let path = Text::new(folder.display().to_string())
        .size(11)
        .font(fonts::JetBrainsMono::REGULAR)
        .color(color::text_muted())
        .width(Fill);
    let row = row![
        column![setting_row::label(lang::t("instance.settings.instance_folder")), path]
            .spacing(3)
            .width(Fill),
        TextButton::ghost(lang::t("common.reveal")).on_press(Message::RevealFolder),
    ]
    .spacing(16)
    .align_y(alignment::Vertical::Center)
    .padding(Padding::new(0.0).top(14).bottom(14));
    divided_panel(vec![row.into()], launch_rows::panel_padding())
}

/// The destructive actions in a red-bordered panel: reinstall the game's files, reset the launch settings, and delete
/// the instance. Each one is answered by the root.
pub fn danger_zone() -> Element<'static, Message> {
    let red = color::buttons::danger_background_hover();
    let delete = row![
        column![
            Text::new(lang::t("instance.settings.delete_title"))
                .size(13)
                .font(fonts::Inter::MEDIUM)
                .color(red),
            Text::new(lang::t("instance.settings.delete_hint"))
                .size(11)
                .font(fonts::Inter::REGULAR)
                .color(color::text_muted()),
        ]
        .spacing(3),
        Space::new().width(Fill),
        TextButton::ghost(lang::t("common.delete"))
            .variant(Variant::Danger)
            .on_press(Message::DeleteInstance),
    ]
    .spacing(16)
    .align_y(alignment::Vertical::Center)
    .padding(Padding::new(0.0).top(14).bottom(14));

    bordered_panel(
        divided(vec![
            described_setting(
                lang::t("instance.settings.reinstall_title"),
                lang::t("instance.settings.reinstall_hint"),
                TextButton::ghost(lang::t("instance.settings.reinstall")).on_press(Message::Reinstall),
            ),
            described_setting(
                lang::t("instance.settings.reset_title"),
                lang::t("instance.settings.reset_hint"),
                TextButton::ghost(lang::t("instance.settings.reset")).on_press(Message::ResetSettings),
            ),
            delete.into(),
        ]),
        launch_rows::panel_padding(),
        anim::fade(red, 0.3),
    )
}
