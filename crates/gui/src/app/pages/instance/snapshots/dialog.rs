//! The diff of a rollback, over the Snapshots tab: each platform mod the snapshot holds against the version the instance has now,
//! a box to tick the ones the rollback changes, and the local jars a full rollback copies back.

use aurora_leaf::Icon;
use chrono::NaiveDateTime;
use iced::widget::{Space, Text, column, row};
use iced::{Element, Fill, alignment};
use lodestone_snapshots::plan::{Item, LocalItem, LocalState, State as Standing};

use super::{Message, State};
use crate::app::components::TextButton;
use crate::app::components::checkbox;
use crate::app::components::modal::Modal;
use crate::app::lang;
use crate::app::theme::{color, fonts};

/// The width of the diff's card.
const WIDTH: f32 = 620.0;

/// When the snapshot was taken, as the diff's title shows it.
fn when_label(when: NaiveDateTime) -> String {
    when.format("%-d %b %Y, %-I:%M %p").to_string()
}

/// The diff over the tab, while a rollback is being picked. `None` when none is open.
pub fn view(state: &State) -> Option<Element<'_, Message>> {
    let prepared = state.compare()?;
    let title = lang::t_with("snapshots.compare.title", &[("when", when_label(prepared.when).as_str())]);
    let mut rows: Vec<Element<'_, Message>> = prepared
        .plan
        .items
        .iter()
        .enumerate()
        .map(|(index, item)| item_row(index, item, state.is_picked(index)))
        .collect();
    if !prepared.plan.local.is_empty() {
        rows.push(
            Text::new(lang::t("snapshots.compare.local"))
                .size(11)
                .font(fonts::Inter::MEDIUM)
                .color(color::text_muted())
                .into(),
        );
        rows.extend(prepared.plan.local.iter().map(local_row));
    }
    let body = column(rows).spacing(6).width(Fill);

    let picked = state.picked_count();
    let roll_back = TextButton::primary(lang::t_with(
        "snapshots.compare.roll_back_selected",
        &[("count", picked.to_string().as_str())],
    ));
    let roll_back = if picked > 0 {
        roll_back.on_press(Message::RollBackSelected)
    } else {
        roll_back.disabled(true)
    };
    let footer = row![
        Space::new().width(Fill),
        TextButton::ghost(lang::t("common.close")).on_press(Message::CloseDialog),
        roll_back,
    ]
    .spacing(12)
    .align_y(alignment::Vertical::Center);

    Some(
        Modal::new(title, body)
            .subtitle(lang::t("snapshots.compare.subtitle"))
            .icon(Icon::lucide().git_compare(), color::primary(), color::primary_tint())
            .footer(footer)
            .on_close(Message::CloseDialog)
            .close_on_scrim(true)
            .width(WIDTH)
            .into(),
    )
}

/// One platform mod of the diff: its box, its name, the versions it goes between, and where it stands.
fn item_row(index: usize, item: &Item, picked: bool) -> Element<'_, Message> {
    let wanted = item.wanted_version.clone().unwrap_or_else(|| item.wanted_id.clone());
    let now = match &item.installed {
        Some(installed) => installed.version.clone().unwrap_or_else(|| installed.version_id.clone()),
        None => lang::t("snapshots.compare.not_installed").to_string(),
    };
    let standing = match item.state {
        Standing::Same => lang::t("snapshots.compare.same"),
        Standing::Changed => lang::t("snapshots.compare.changed"),
        Standing::NotInstalled => lang::t("snapshots.compare.not_installed"),
    };
    // A mod already at the snapshot's release has no box to tick, so its mark only shows that it is kept.
    let mark: Element<'_, Message> = match item.state {
        Standing::Same => checkbox::mark(checkbox::State::Unchecked),
        _ => {
            let state = if picked { checkbox::State::Checked } else { checkbox::State::Unchecked };
            checkbox::checkbox(state, Message::Toggle(index))
        }
    };
    row![
        mark,
        column![
            Text::new(item.name.as_str())
                .size(13)
                .font(fonts::Inter::MEDIUM)
                .color(color::text_default()),
            Text::new(format!("{wanted} → {now}"))
                .size(11)
                .font(fonts::JetBrainsMono::REGULAR)
                .color(color::text_muted()),
        ]
        .spacing(3)
        .width(Fill),
        Text::new(standing).size(11).font(fonts::Inter::MEDIUM).color(color::text_muted()),
    ]
    .spacing(12)
    .align_y(alignment::Vertical::Center)
    .into()
}

/// One local jar of the diff: its file name, and what a rollback does with it.
fn local_row(jar: &LocalItem) -> Element<'_, Message> {
    let standing = match jar.state {
        LocalState::Same => lang::t("snapshots.compare.local_same"),
        LocalState::Missing => lang::t("snapshots.compare.local_missing"),
        LocalState::Differs => lang::t("snapshots.compare.local_differs"),
    };
    row![
        Text::new(jar.file.as_str())
            .size(13)
            .font(fonts::Inter::MEDIUM)
            .color(color::text_default())
            .width(Fill),
        Text::new(standing).size(11).font(fonts::Inter::MEDIUM).color(color::text_muted()),
    ]
    .spacing(12)
    .align_y(alignment::Vertical::Center)
    .into()
}
