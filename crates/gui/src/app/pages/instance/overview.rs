//! The Overview tab: About, sync progress and a screenshot strip beside the
//! session telemetry and the players.
//!
//! Telemetry and the sync figures are demo state until the game launcher is
//! wired in; the instance's own name, version and folder are real.

use aurora_leaf::Icon;
use iced::widget::{Space, Text, column, container, row};
use iced::{Background, Border, Color, Element, Fill, Length, Padding, alignment};
use iced_color_extension::rgb;

use crate::app::components::grid::grid;
use crate::app::components::panel::panel;
use crate::app::components::screenshot_tile::{Caption, screenshot_tile};
use crate::app::components::{ProgressBar, PulseDot};
use crate::app::pages::library::instance::{Biome, Instance, InstanceState};
use crate::app::theme::{color, fonts, utility};

use super::format::format_playtime;

/// The strip's tiles: biome, art seed and file name.
const STRIP: [(Biome, u64, &str); 6] = [
    (Biome::End, 0, "2026-04-10_18.00.png"),
    (Biome::Nether, 1, "2026-04-11_18.01.png"),
    (Biome::Mushroom, 2, "2026-04-12_18.02.png"),
    (Biome::Ocean, 3, "2026-04-13_18.03.png"),
    (Biome::Cherry, 4, "2026-04-14_18.04.png"),
    (Biome::Snow, 5, "2026-04-15_18.05.png"),
];

/// The players in the session, with their avatar initials and tint.
const PLAYERS: [(&str, &str, Color); 3] = [
    ("PI", "pixelpete", color::Accents::PINK),
    ("CA", "cavesworn", color::Accents::CYAN),
    ("SU", "sundayknight", color::Accents::AMBER),
];

/// The Overview's two columns: the main column on the left, telemetry on the right.
pub fn view<'a, M: 'static>(instance: &'a Instance) -> Element<'a, M> {
    let left = column![about_panel(instance), sync_panel::<M>(), screenshot_strip::<M>()]
        .spacing(16)
        .width(Length::FillPortion(2));
    let right = column![session_panel::<M>(instance), players_panel::<M>()]
        .spacing(14)
        .width(Length::FillPortion(1));

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

fn about_text(instance: &Instance) -> String {
    if instance.name == "Aether Legacy" {
        "A re-imagining of the classic Aether mod — soar between floating isles, battle Slider bosses, and uncover \
         the secrets of a continent suspended in the void."
            .to_string()
    } else {
        format!(
            "A {} instance running Minecraft {}{}. Launch it to continue where you left off — session telemetry, \
             screenshots and friends will appear here while it is running.",
            instance.loader.label(),
            instance.mc_version,
            instance
                .loader_version
                .as_ref()
                .map(|version| format!(" with {} {version}", instance.loader.label()))
                .unwrap_or_default()
        )
    }
}

fn sync_panel<M: 'static>() -> Element<'static, M> {
    panel(
        column![
            row![
                utility::icon(Icon::lucide().download(), 14.0, Some((color::Primary::DEFAULT, color::Primary::DEFAULT))),
                Text::new("Syncing mod updates")
                    .size(13)
                    .font(fonts::Inter::SEMI_BOLD)
                    .color(color::Text::HEADLINE),
                Space::new().width(Fill),
                Text::new("63%")
                    .size(11)
                    .font(fonts::JetBrainsMono::SEMI_BOLD)
                    .color(color::Primary::DEFAULT),
            ]
            .spacing(10)
            .align_y(alignment::Vertical::Center),
            ProgressBar::new(0.63).height(6),
            Text::new("Downloading iris-mc1.20.1-1.7.1.jar · 12.4 MB/s")
                .size(11)
                .font(fonts::JetBrainsMono::REGULAR)
                .color(color::Text::PLACEHOLDER),
        ]
        .spacing(10),
    )
}

/// The six-tile strip of recent captures, three to a row.
fn screenshot_strip<M: 'static>() -> Element<'static, M> {
    let tiles = STRIP
        .iter()
        .map(|&(biome, seed, name)| screenshot_tile(biome, seed, Caption::Overlay(name), 100.0))
        .collect();

    column![
        Text::new("Screenshots")
            .size(13)
            .font(fonts::Inter::SEMI_BOLD)
            .color(color::Text::HEADLINE),
        grid(tiles, 3, 10.0),
    ]
    .spacing(10)
    .into()
}

fn session_panel<M: 'static>(instance: &Instance) -> Element<'static, M> {
    let playing = instance.state == InstanceState::Playing;
    let playtime = if playing { "12 min".to_string() } else { "—".to_string() };
    let memory = if playing { "2.1 / 6 GB" } else { "—" };
    let framerate = if playing { "58 fps" } else { "—" };
    let tick = if playing { "18.2 ms" } else { "—" };

    panel(
        column![
            section_label("SESSION"),
            Space::new().height(2),
            session_row("Playtime", playtime, color::Primary::DEFAULT),
            session_row("Total", format_playtime(instance.playtime_seconds), color::Text::HEADLINE),
            session_row("Memory", memory.to_string(), color::Accents::CYAN),
            session_row("Framerate", framerate.to_string(), color::Accents::AMBER),
            session_row("Tick time", tick.to_string(), color::Text::DEFAULT),
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

fn players_panel<M: 'static>() -> Element<'static, M> {
    panel(
        column![
            section_label("PLAYING WITH"),
            Space::new().height(2),
            column(PLAYERS.into_iter().map(player_row::<M>)).spacing(0),
        ]
        .spacing(0),
    )
}

fn player_row<M: 'static>((initials, name, ink): (&'static str, &'static str, Color)) -> Element<'static, M> {
    row![
        container(Text::new(initials).size(10).font(fonts::Inter::EXTRA_BOLD).color(rgb!(0, 0, 0, 100%)),)
            .width(24)
            .height(24)
            .align_x(alignment::Horizontal::Center)
            .align_y(alignment::Vertical::Center)
            .style(move |_| container::Style {
                background: Some(Background::Color(ink)),
                border: Border {
                    radius: 6.0.into(),
                    ..Border::default()
                },
                ..container::Style::default()
            }),
        Text::new(name).size(12).font(fonts::Inter::MEDIUM).color(color::Text::DEFAULT),
        Space::new().width(Fill),
        PulseDot::with_color(color::Primary::DEFAULT).size(6.0),
    ]
    .spacing(10)
    .align_y(alignment::Vertical::Center)
    .padding(Padding::new(0.0).top(6).bottom(6))
    .into()
}
