//! The bell and its panel: the unread count on the bell, and the panel it opens below it. Each row shows a notification's
//! text, its buttons, and the control that dismisses it.

use std::path::PathBuf;

use aurora_leaf::Icon;
use chrono::{DateTime, Local};
use iced::widget::{Column, Space, Text, column, container, row, stack};
use iced::{Background, Border, Element, Fill, Length, Padding, Shadow, Vector, alignment};
use iced_color_extension::rgb;

use super::model::{Kind, Notification};
use super::{Action, Message, Notifications};
use crate::app::components::hint::hint;
use crate::app::components::icon_button::{self, IconButton, ghost_button_style};
use crate::app::components::popover::{Placement, Popover};
use crate::app::components::{TextButton, scrollbar};
use crate::app::lang;
use crate::app::theme::{blur, color, fonts};

/// The panel's width, as the Downloads popover has.
const WIDTH: f32 = 380.0;
/// The tallest the list grows before it scrolls.
const LIST_HEIGHT: f32 = 420.0;
/// The most a count on the bell shows before it reads "9+".
const COUNT_LIMIT: usize = 9;

/// The bell: its icon, the count of unread notifications on its corner, and the panel it opens below it.
pub fn bell(state: &Notifications) -> Element<'_, Message> {
    let unread = state.unread();
    let button = hint(
        IconButton::new(Icon::lucide().bell())
            .size(36)
            .radius(12.0)
            .icon_size(16.0)
            .style(ghost_button_style())
            .on_press(Message::TogglePanel),
        lang::t("notifications.bell"),
    );
    let face: Element<'_, Message> = if unread > 0 { stack![button, count_chip(unread)].into() } else { button };
    // The panel is only built while it shows, since the header redraws with every frame.
    let panel: Element<'_, Message> = if state.is_open() { view(state) } else { Space::new().into() };
    Popover::new(face, panel)
        .open(state.is_open())
        .on_dismiss(Message::DismissPanel)
        .placement(Placement::Below)
        .into()
}

/// The unread count on the bell's top-right corner.
fn count_chip(count: usize) -> Element<'static, Message> {
    let label = if count > COUNT_LIMIT {
        format!("{COUNT_LIMIT}+")
    } else {
        count.to_string()
    };
    let chip = container(Text::new(label).size(10).font(fonts::JetBrainsMono::SEMI_BOLD).color(color::primary_on()))
        .padding(Padding::new(0.0).left(5).right(5).top(1).bottom(1))
        .style(|_| container::Style {
            background: Some(Background::Color(color::primary())),
            border: Border {
                radius: 999.0.into(),
                ..Border::default()
            },
            ..container::Style::default()
        });
    container(chip)
        .width(Fill)
        .height(Fill)
        .align_x(alignment::Horizontal::Right)
        .align_y(alignment::Vertical::Top)
        .into()
}

/// The panel: a title with the controls that mark or clear the list, then the list, newest first.
fn view(state: &Notifications) -> Element<'_, Message> {
    let header = row![
        Text::new(lang::t("notifications.title"))
            .size(14)
            .font(fonts::Inter::SEMI_BOLD)
            .color(color::text_headline()),
        Space::new().width(Fill),
        TextButton::ghost(lang::t("notifications.mark_all_read")).on_press(Message::MarkAllRead),
        TextButton::ghost(lang::t("notifications.clear")).on_press(Message::ClearAll),
    ]
    .spacing(8)
    .align_y(alignment::Vertical::Center);
    let body: Element<'_, Message> = if state.items().is_empty() {
        container(
            Text::new(lang::t("notifications.empty"))
                .size(13)
                .font(fonts::Inter::REGULAR)
                .color(color::text_muted()),
        )
        .padding(Padding::new(12.0))
        .width(Fill)
        .into()
    } else {
        let rows = Column::with_children(state.items().iter().map(row_view)).spacing(8).width(Fill);
        // The cards end where the scrollbar's track begins, as the Discover sidebar's do, so the track never sits on a card.
        let rows = container(rows).padding(Padding::new(0.0).right(scrollbar::TRACK_WIDTH)).width(Fill);
        container(scrollbar::scroll_panel(rows).width(Fill).height(Length::Shrink))
            .max_height(LIST_HEIGHT)
            .width(Fill)
            .into()
    };
    container(column![header, rule(), body].spacing(12).width(Fill))
        .padding(14)
        .width(WIDTH)
        .style(|_| container::Style {
            background: Some(Background::Color(color::surface_bg3())),
            border: Border {
                radius: 14.0.into(),
                width: 1.0,
                color: color::surface_line_strong(),
            },
            shadow: Shadow {
                color: rgb!(0, 0, 0, 45%),
                offset: Vector::new(0.0, 10.0),
                blur_radius: blur(28.0),
            },
            ..container::Style::default()
        })
        .into()
}

/// A one-pixel hairline between the panel's sections.
fn rule() -> Element<'static, Message> {
    container(Space::new().width(Fill).height(1))
        .style(|_| container::Style {
            background: Some(Background::Color(color::surface_line())),
            ..container::Style::default()
        })
        .into()
}

/// One notification: its unread dot, its title, body and age, its buttons, and the control that dismisses it.
fn row_view(item: &Notification) -> Element<'_, Message> {
    let unread = !item.read;
    let dot = container(Space::new().width(6).height(6)).style(move |_| container::Style {
        background: unread.then(|| Background::Color(color::primary())),
        border: Border {
            radius: 3.0.into(),
            ..Border::default()
        },
        ..container::Style::default()
    });
    let dismiss = hint(
        IconButton::new(Icon::lucide().x())
            .size(24)
            .radius(8.0)
            .icon_size(12.0)
            .style(icon_button::secondary_button_style())
            .on_press(Message::Dismiss(item.id)),
        lang::t("notifications.dismiss"),
    );
    let text = column![
        Text::new(title_of(item))
            .size(13)
            .font(fonts::Inter::SEMI_BOLD)
            .color(color::text_headline()),
        Text::new(body_of(item)).size(12).font(fonts::Inter::REGULAR).color(color::text_muted()),
        Text::new(ago(item.created_at, Local::now()))
            .size(11)
            .font(fonts::JetBrainsMono::REGULAR)
            .color(color::text_placeholder()),
    ]
    .spacing(2)
    .width(Fill);
    let mut buttons = row![].spacing(8);
    for (label, action) in actions(item) {
        buttons = buttons.push(TextButton::ghost(lang::t(label)).on_press(Message::Act(action)));
    }
    container(column![row![dot, text, dismiss].spacing(10).align_y(alignment::Vertical::Top), buttons,].spacing(8))
        .padding(10)
        .width(Fill)
        .style(|_| container::Style {
            background: Some(Background::Color(rgb!(255, 255, 255, 4%))),
            border: Border {
                radius: 10.0.into(),
                width: 1.0,
                color: color::surface_line(),
            },
            ..container::Style::default()
        })
        .into()
}

/// The title a notification's row shows, in the active language.
fn title_of(item: &Notification) -> String {
    let subject = item.subject.as_str();
    match item.kind {
        Kind::Crash => lang::t_with("notifications.crash.title", &[("name", subject)]),
        Kind::ModpackUpdate => lang::t_with("notifications.modpack.title", &[("name", subject)]),
        Kind::ModUpdate => lang::t_with("notifications.mods.title", &[("name", subject)]),
        Kind::LauncherUpdate => lang::t_with("notifications.launcher.title", &[("version", subject)]),
        Kind::InstallDone => lang::t_with("notifications.install.done.title", &[("name", subject)]),
        Kind::InstallFailed => lang::t_with("notifications.install.failed.title", &[("name", subject)]),
        Kind::PackUpdated => lang::t_with("notifications.pack_update.done.title", &[("name", subject)]),
        Kind::ExportDone => lang::t_with("notifications.export.done.title", &[("name", subject)]),
        Kind::ExportFailed => lang::t_with("notifications.export.failed.title", &[("name", subject)]),
    }
}

/// The body a notification's row shows, in the active language. Its detail fills the body's blank.
fn body_of(item: &Notification) -> String {
    let detail = item.detail.as_str();
    match item.kind {
        Kind::Crash => lang::t_with("launch.exited_with_error", &[("code", detail)]),
        Kind::ModpackUpdate => lang::t_with("notifications.modpack.body", &[("version", detail)]),
        Kind::ModUpdate => lang::t_count("notifications.mods.body", detail.parse().unwrap_or(0)),
        Kind::LauncherUpdate => lang::t("notifications.launcher.body").to_owned(),
        Kind::InstallDone => lang::t("notifications.install.done.body").to_owned(),
        Kind::InstallFailed => lang::t_with("notifications.install.failed.body", &[("error", detail)]),
        Kind::PackUpdated => lang::t("notifications.pack_update.done.body").to_owned(),
        Kind::ExportDone => lang::t("notifications.export.done.body").to_owned(),
        Kind::ExportFailed => lang::t_with("notifications.export.failed.body", &[("error", detail)]),
    }
}

/// The buttons a notification shows, each with the key of its label and the action it starts. A kind with nothing
/// to open shows none.
fn actions(item: &Notification) -> Vec<(&'static str, Action)> {
    let instance = item.instance_id.clone();
    match item.kind {
        Kind::Crash => instance
            .map(|id| ("notifications.action.view_log", Action::Logs(id)))
            .into_iter()
            .chain(
                item.target
                    .clone()
                    .map(|report| ("notifications.action.open_report", Action::OpenFile(PathBuf::from(report)))),
            )
            .collect(),
        Kind::ModpackUpdate => instance
            .map(|id| ("notifications.action.update", Action::Versions(id)))
            .into_iter()
            .chain(
                item.target
                    .as_deref()
                    .and_then(|target| target.split_once('/'))
                    .map(|(platform, project)| {
                        (
                            "notifications.action.open_modpack",
                            Action::Modpack {
                                platform: platform.to_owned(),
                                project: project.to_owned(),
                                name: item.subject.clone(),
                            },
                        )
                    }),
            )
            .collect(),
        Kind::ModUpdate => instance
            .map(|id| ("notifications.action.review_mods", Action::Mods(id)))
            .into_iter()
            .collect(),
        Kind::LauncherUpdate => vec![("notifications.action.update", Action::Update)],
        Kind::InstallDone | Kind::PackUpdated => instance
            .map(|id| ("notifications.action.open_instance", Action::Instance(id)))
            .into_iter()
            .collect(),
        Kind::ExportDone => item
            .target
            .clone()
            .map(|path| ("notifications.action.show_file", Action::ShowFolder(PathBuf::from(path))))
            .into_iter()
            .collect(),
        Kind::InstallFailed | Kind::ExportFailed => Vec::new(),
    }
}

/// How long ago a notification was raised, in words, from `now`.
fn ago(created: DateTime<Local>, now: DateTime<Local>) -> String {
    let minutes = usize::try_from((now - created).num_minutes()).unwrap_or(0);
    match minutes {
        0 => lang::t("notifications.ago.now").to_owned(),
        1..60 => lang::t_count("notifications.ago.minutes", minutes),
        60..1440 => lang::t_count("notifications.ago.hours", minutes / 60),
        _ => lang::t_count("notifications.ago.days", minutes / 1440),
    }
}

#[cfg(test)]
#[path = "../../../tests/app/notifications/panel.rs"]
mod tests;
