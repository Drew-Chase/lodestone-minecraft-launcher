//! The launcher picker's rows: one section per launcher, with the folder it was found in, a Locate button, a select-all checkbox,
//! and a row per instance it lists. An instance that cannot be imported shows its reason and cannot be checked.

use std::collections::BTreeSet;

use iced::widget::{Text, column, row};
use iced::{Color, Element, Fill, alignment};

use super::super::launcher::{Instance, Launcher, Pick, Scan};
use super::Message;
use crate::app::components::button::TextButton;
use crate::app::components::checkbox::{self, checkbox};
use crate::app::lang;
use crate::app::theme::{color, fonts};

/// The launchers whose formats have not yet been checked against a real install. Their sections say so.
const UNVERIFIED: [Launcher; 4] = [Launcher::Prism, Launcher::MultiMc, Launcher::AtLauncher, Launcher::Minecraft];

/// The tab's content: a section per launcher. While the launchers are looked for, a line says so, and an empty scan says so.
pub(super) fn rows(scans: &[Scan], chosen: &BTreeSet<Pick>, scanning: bool, busy: bool) -> Element<'static, Message> {
    if scans.is_empty() {
        let text = if scanning {
            lang::t("import.launcher.looking")
        } else {
            lang::t("import.launcher.none")
        };
        return small(text.to_string(), color::text_muted());
    }
    column(scans.iter().map(|scan| section(scan, chosen, busy)))
        .spacing(18)
        .width(Fill)
        .into()
}

/// One launcher: its name, where it was found, the Locate button, a select-all checkbox, and a row per instance.
fn section(scan: &Scan, chosen: &BTreeSet<Pick>, busy: bool) -> Element<'static, Message> {
    let launcher = scan.launcher;
    let mut heading = column![
        Text::new(launcher.name())
            .size(14)
            .font(fonts::Inter::SEMI_BOLD)
            .color(color::text_headline()),
        status(scan),
    ]
    .spacing(3)
    .width(Fill);
    if UNVERIFIED.contains(&launcher) {
        heading = heading.push(small(lang::t("import.launcher.unverified").to_string(), color::text_muted()));
    }
    let mut header = row![heading].spacing(12).align_y(alignment::Vertical::Center);
    let importable: Vec<&Instance> = scan.instances.iter().filter(|instance| instance.summary.is_ok()).collect();
    if !importable.is_empty() {
        let picked = importable.iter().filter(|instance| chosen.contains(&pick_of(instance))).count();
        let state = match picked {
            0 => checkbox::State::Unchecked,
            count if count == importable.len() => checkbox::State::Checked,
            _ => checkbox::State::Partial,
        };
        header = header.push(
            row![
                checkbox(state, Message::ToggleLauncher(launcher)),
                small(lang::t("import.launcher.select_all").to_string(), color::text_muted()),
            ]
            .spacing(8)
            .align_y(alignment::Vertical::Center),
        );
    }
    if !busy {
        header = header.push(
            TextButton::ghost(lang::t("import.launcher.locate"))
                .padding(iced::Padding::new(0.0).left(12).right(12).top(7).bottom(7))
                .on_press(Message::Locate(launcher)),
        );
    }
    let mut section = column![header].spacing(8).width(Fill);
    for instance in &scan.instances {
        section = section.push(instance_row(instance, chosen));
    }
    section.into()
}

/// Where the folder of a launcher was found, or that it was not, or the error that stopped the look.
fn status(scan: &Scan) -> Element<'static, Message> {
    if let Some(error) = &scan.error {
        return small(error.clone(), color::pink());
    }
    match &scan.folder {
        Some(folder) => {
            let shown = folder.to_string_lossy();
            small(
                lang::t_with("import.launcher.found_at", &[("folder", shown.as_ref())]).to_string(),
                color::text_muted(),
            )
        }
        None => small(lang::t("import.launcher.not_found").to_string(), color::text_muted()),
    }
}

/// One instance: its checkbox, its name, and its Minecraft version and loader, or the reason it cannot be imported.
fn instance_row(instance: &Instance, chosen: &BTreeSet<Pick>) -> Element<'static, Message> {
    let pick = pick_of(instance);
    let state = if chosen.contains(&pick) {
        checkbox::State::Checked
    } else {
        checkbox::State::Unchecked
    };
    let (detail, ink) = match &instance.summary {
        Ok(summary) => (summary.clone(), color::text_muted()),
        Err(reason) => (reason.clone(), color::pink()),
    };
    row![
        checkbox(state, Message::ToggleInstance(pick)),
        column![
            Text::new(instance.name.clone())
                .size(13)
                .font(fonts::Inter::SEMI_BOLD)
                .color(color::text_default()),
            Text::new(detail).size(11).font(fonts::JetBrainsMono::REGULAR).color(ink),
        ]
        .spacing(2)
        .width(Fill),
    ]
    .spacing(12)
    .align_y(alignment::Vertical::Center)
    .into()
}

/// The pick an instance row stands for.
fn pick_of(instance: &Instance) -> Pick {
    Pick {
        launcher: instance.launcher,
        source: instance.source.clone(),
    }
}

/// A line of small text in `ink`.
fn small(text: String, ink: Color) -> Element<'static, Message> {
    Text::new(text).size(12).font(fonts::Inter::REGULAR).color(ink).into()
}
