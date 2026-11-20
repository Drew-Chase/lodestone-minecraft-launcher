//! The Game Version, Paths & Files and Danger Zone sections. The launch sections are built from the shared rows in
//! `components::launch_rows`, and the General section in `general.rs`.

use std::path::Path;

use iced::widget::{Space, Text, column, container, row};
use iced::{Element, Fill, Padding, alignment};
use lodestone_loaders::LoaderVersion;
use lodestone_snapshots::model::{LIMIT_MAX, LIMIT_MIN};

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

/// The Game Version rows: the Minecraft version and the loader build, the loader's type, the snapshots toggle, and the
/// mod updates switch.
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
        .disabled(instance.locked)
        .width(180);

    let mut rows = vec![setting(lang::t("instance.settings.minecraft"), minecraft)];
    if instance.locked {
        rows.insert(0, locked_row());
    }

    if instance.loader == Loader::Vanilla {
        let (ink, fill) = loader_palette(instance.loader);
        let badge = Badge::new(instance.loader.label(), ink, fill).with_loader(instance.loader);
        rows.push(setting(lang::t("instance.settings.mod_loader"), badge));
    } else {
        rows.push(setting(lang::t("instance.settings.mod_loader"), loader_control(instance, builds)));
    }

    let snapshot_toggle = Toggle::new(snapshots).on_toggle(Message::Edit(Edit::Snapshots(!snapshots)));
    rows.push(setting(lang::t("instance.settings.snapshots"), snapshot_toggle));

    let updates_on = !instance.settings.mod_updates_disabled;
    let updates_toggle = Toggle::new(updates_on).on_toggle(Message::Edit(Edit::ModUpdates(!updates_on)));
    rows.push(described_setting(
        lang::t("instance.settings.mod_updates"),
        lang::t("instance.settings.mod_updates_hint"),
        updates_toggle,
    ));

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
        .disabled(instance.locked)
        .width(180);
    row![badge, build].spacing(10).align_y(alignment::Vertical::Center).into()
}

/// The row that says why the versions cannot change: the instance is locked to its modpack.
fn locked_row<'a>() -> Element<'a, Message> {
    container(
        Text::new(lang::t("instance.settings.locked"))
            .size(12)
            .font(fonts::Inter::REGULAR)
            .color(color::text_muted()),
    )
    .padding(Padding::new(0.0).top(14).bottom(14))
    .into()
}

/// A row that names a list's error.
fn error_row<'a>(error: &'a str) -> Element<'a, Message> {
    Text::new(error)
        .size(11)
        .font(fonts::Inter::REGULAR)
        .color(color::buttons::danger_background_hover())
        .into()
}

/// The instance folder, a button that opens it, and how many mod snapshots the instance keeps.
pub fn paths_files(folder: &Path, snapshot_limit: Option<u32>, default_limit: u32) -> Element<'_, Message> {
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
    let snapshots = container(snapshot_limit_row(snapshot_limit, default_limit)).padding(Padding::new(0.0).top(14).bottom(14));
    divided_panel(vec![row.into(), snapshots.into()], launch_rows::panel_padding())
}

/// How many mod snapshots the instance keeps. The first option takes the global default, which the label names, and the
/// others set the instance's own limit.
fn snapshot_limit_row<'a>(limit: Option<u32>, default_limit: u32) -> Element<'a, Message> {
    let default = lang::t_with("snapshots.limit.default", &[("count", default_limit.to_string().as_str())]);
    let labels = std::iter::once(default).chain((LIMIT_MIN..=LIMIT_MAX).map(launch_rows::snapshot_limit_label));
    let selected = limit.map_or(0, |value| value as usize + 1);
    let select = Select::new(labels, Some(selected))
        .on_select(|index| {
            let limit = index.checked_sub(1).map(|value| value as u32);
            Message::Edit(Edit::SnapshotLimit(limit))
        })
        .width(180);
    described_setting(
        lang::t("instance.settings.snapshots_limit"),
        lang::t("instance.settings.snapshots_limit_hint"),
        select,
    )
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
