//! The Overview tab: About and a strip of the newest captures beside the
//! session readout.
//!
//! Only what the instance folder or the database holds is shown. The strip's
//! thumbnails come from the screenshots tab's cache.

use aurora_leaf::Icon;
use iced::widget::{Column, Space, Text, column, container, row};
use iced::{Background, Color, Element, Fill, Length, Padding, alignment};

use super::format::format_playtime;
use super::{Message, screenshots};
use crate::app::components::ProgressBar;
use crate::app::components::grid::grid;
use crate::app::components::instance_card::install_status;
use crate::app::components::panel::panel;
use crate::app::components::screenshot_tile::Caption;
use crate::app::downloads::{Pending, progress_detail};
use crate::app::lang;
use crate::app::pages::library::instance::{Instance, Loader};
use crate::app::theme::{color, fonts, utility};

/// How many of the newest captures the strip shows, three to a row.
const STRIP_LEN: usize = 6;

/// The Overview's two columns: About and the strip on the left, the session
/// readout on the right. While the instance installs, its install panel leads the left column.
pub fn view<'a>(instance: &'a Instance, screenshots: &'a screenshots::State, install: Option<&'a Pending>) -> Element<'a, Message> {
    let mut left = Column::new().spacing(16).width(Length::FillPortion(2));
    if let Some(pending) = install {
        left = left.push(install_panel::<Message>(pending));
    }
    left = left.push(about_panel(instance)).push(screenshot_strip(screenshots));
    let right = column![session_panel::<Message>(instance)].spacing(14).width(Length::FillPortion(1));

    row![left, right].spacing(18).align_y(alignment::Vertical::Top).width(Fill).into()
}

/// The panel of an install that has not ended: its stage and percent, the bar, and what it is doing.
fn install_panel<M: 'static>(pending: &Pending) -> Element<'static, M> {
    let fraction = pending.progress.as_ref().and_then(|progress| progress.fraction);
    panel(
        column![
            row![
                utility::icon(Icon::lucide().download(), 14.0, Some((color::Primary::DEFAULT, color::Primary::DEFAULT))),
                Text::new(install_status(pending))
                    .size(13)
                    .font(fonts::Inter::SEMI_BOLD)
                    .color(color::Text::HEADLINE),
            ]
            .spacing(8)
            .align_y(alignment::Vertical::Center),
            ProgressBar::new(fraction.unwrap_or(0.0)).indeterminate(fraction.is_none()).width(Fill),
            Text::new(progress_detail(pending.progress.as_ref()))
                .size(11)
                .font(fonts::JetBrainsMono::REGULAR)
                .color(color::Text::MUTED),
        ]
        .spacing(10),
    )
}

fn about_panel<M: 'static>(instance: &Instance) -> Element<'static, M> {
    panel(
        column![
            Text::new(lang::t("instance.overview.about"))
                .size(13)
                .font(fonts::Inter::SEMI_BOLD)
                .color(color::Text::HEADLINE),
            Text::new(about_text(instance))
                .size(12)
                .line_height(1.6)
                .font(fonts::Inter::REGULAR)
                .color(color::Text::MUTED)
                .width(Fill),
        ]
        .spacing(10),
    )
}

/// The loader and the Minecraft version the instance runs.
fn about_text(instance: &Instance) -> String {
    if instance.loader == Loader::Vanilla {
        return lang::t_with("instance.overview.vanilla", &[("version", instance.mc_version.as_str())]);
    }
    match &instance.loader_version {
        Some(version) => lang::t_with(
            "instance.overview.modded",
            &[
                ("loader", instance.loader.label()),
                ("loader_version", version.as_str()),
                ("version", instance.mc_version.as_str()),
            ],
        ),
        None => lang::t_with(
            "instance.overview.modded_no_version",
            &[("loader", instance.loader.label()), ("version", instance.mc_version.as_str())],
        ),
    }
}

/// The newest captures, three to a row. A press opens the viewer on a capture, as the Screenshots tab
/// does. Until one is read, the strip says so.
fn screenshot_strip<'a>(screenshots: &'a screenshots::State) -> Element<'a, Message> {
    let body: Element<'a, Message> = if screenshots.shots().is_empty() {
        Text::new(lang::t("instance.overview.no_screenshots"))
            .size(12)
            .font(fonts::Inter::REGULAR)
            .color(color::Text::MUTED)
            .into()
    } else {
        let tiles = screenshots
            .shots()
            .iter()
            .take(STRIP_LEN)
            .enumerate()
            .map(|(index, shot)| screenshots::clickable(screenshots, index, Caption::Overlay(&shot.file_name), 100.0).map(Message::Screenshots))
            .collect();
        grid(tiles, 3, 10.0)
    };

    column![
        Text::new(lang::t("instance.overview.screenshots"))
            .size(13)
            .font(fonts::Inter::SEMI_BOLD)
            .color(color::Text::HEADLINE),
        body,
    ]
    .spacing(10)
    .into()
}

/// The session readout. Only the stored playtime is real so far; live figures
/// need the game to report them.
fn session_panel<M: 'static>(instance: &Instance) -> Element<'static, M> {
    panel(
        column![
            section_label(lang::t("instance.overview.session")),
            Space::new().height(2),
            session_row(
                lang::t("instance.overview.total"),
                format_playtime(instance.playtime_seconds),
                color::Text::HEADLINE
            ),
        ]
        .spacing(0),
    )
}

fn session_row<M: 'static>(label: &'static str, value: String, ink: Color) -> Element<'static, M> {
    column![
        container(
            row![
                Text::new(label).size(12).font(fonts::Inter::REGULAR).color(color::Text::MUTED),
                Space::new().width(Fill),
                Text::new(value).size(12).font(fonts::JetBrainsMono::SEMI_BOLD).color(ink),
            ]
            .align_y(alignment::Vertical::Center),
        )
        .width(Fill)
        .padding(Padding::new(0.0).top(8).bottom(8)),
        container(Space::new().width(Fill).height(1)).width(Fill).style(|_| container::Style {
            background: Some(Background::Color(color::Surface::LINE)),
            ..container::Style::default()
        }),
    ]
    .into()
}

fn section_label<M: 'static>(label: &'static str) -> Element<'static, M> {
    Text::new(label)
        .size(12)
        .font(fonts::JetBrainsMono::SEMI_BOLD)
        .color(color::Text::PLACEHOLDER)
        .into()
}
