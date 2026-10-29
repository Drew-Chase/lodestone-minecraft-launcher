//! The Overview tab: About and a strip of the newest captures beside the
//! session readout.
//!
//! Only what the instance folder or the database holds is shown. The strip's
//! thumbnails come from the screenshots tab's cache.

use iced::widget::{Space, Text, column, container, row};
use iced::{Background, Color, Element, Fill, Length, Padding, alignment};

use super::format::format_playtime;
use super::screenshots;
use crate::app::components::grid::grid;
use crate::app::components::panel::panel;
use crate::app::components::screenshot_tile::{Caption, photo_tile};
use crate::app::pages::library::instance::{Instance, Loader};
use crate::app::theme::{color, fonts};

/// How many of the newest captures the strip shows, three to a row.
const STRIP_LEN: usize = 6;

/// The Overview's two columns: About and the strip on the left, the session
/// readout on the right.
pub fn view<'a, M: 'static>(instance: &'a Instance, screenshots: &'a screenshots::State) -> Element<'a, M> {
    let left = column![about_panel(instance), screenshot_strip::<M>(screenshots)]
        .spacing(16)
        .width(Length::FillPortion(2));
    let right = column![session_panel::<M>(instance)].spacing(14).width(Length::FillPortion(1));

    row![left, right].spacing(18).align_y(alignment::Vertical::Top).width(Fill).into()
}

fn about_panel<M: 'static>(instance: &Instance) -> Element<'static, M> {
    panel(
        column![
            Text::new("About").size(13).font(fonts::Inter::SEMI_BOLD).color(color::Text::HEADLINE),
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
        return format!("Vanilla Minecraft {}.", instance.mc_version);
    }
    match &instance.loader_version {
        Some(version) => format!("{} {version} on Minecraft {}.", instance.loader.label(), instance.mc_version),
        None => format!("{} on Minecraft {}.", instance.loader.label(), instance.mc_version),
    }
}

/// The newest captures, three to a row. Until one is read, the strip says so.
fn screenshot_strip<'a, M: 'a>(screenshots: &'a screenshots::State) -> Element<'a, M> {
    let body: Element<'a, M> = if screenshots.shots().is_empty() {
        Text::new("No screenshots yet.")
            .size(12)
            .font(fonts::Inter::REGULAR)
            .color(color::Text::MUTED)
            .into()
    } else {
        let tiles = screenshots
            .shots()
            .iter()
            .take(STRIP_LEN)
            .map(|shot| photo_tile(screenshots.thumbnail(&shot.path), Caption::Overlay(&shot.file_name), 100.0))
            .collect();
        grid(tiles, 3, 10.0)
    };

    column![
        Text::new("Screenshots")
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
            section_label("SESSION"),
            Space::new().height(2),
            session_row("Total", format_playtime(instance.playtime_seconds), color::Text::HEADLINE),
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
