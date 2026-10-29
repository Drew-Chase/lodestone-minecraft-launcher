//! The Library hero banner for the most-recent instance, and the quick-action
//! cards beneath it.

use crate::app::components::scene_art::hero_handle;
use crate::app::components::{IconButton, PulseDot, TextButton};
use crate::app::theme::{color, fonts};
use aurora_leaf::Icon;
use chrono::{DateTime, Local};
use iced::widget::{Text, column, container, mouse_area, row, stack, svg};
use iced::{Background, Border, Color, Element, Fill, Padding, alignment};
use iced_color_extension::rgb;

use super::instance::{InstanceState, Loader};
use super::{Library, Message, chip, icon};

impl Library {
    /// The most-recent instance's wide banner card — rounded with a
    /// hairline border, scene art behind a left-dark readability gradient.
    /// `None` without instances, so the page reserves no room for it.
    pub(super) fn hero(&self) -> Option<Element<'_, Message>> {
        let hero = self.instances.iter().max_by_key(|instance| instance.recency().timestamp_millis())?;
        let playing = hero.state == InstanceState::Playing;

        let chips = row![
            status_chip(hero.state),
            chip(
                match &hero.loader_version {
                    Some(version) => format!("{} {version}", hero.loader.label()),
                    None => hero.loader.label().to_string(),
                },
                match hero.loader {
                    Loader::Vanilla => color::Primary::DEFAULT,
                    Loader::Fabric => color::Accents::VIOLET_LIGHT,
                    Loader::Forge => color::Accents::AMBER,
                    Loader::Quilt => color::Accents::CYAN,
                    Loader::NeoForge => color::Accents::PINK,
                },
                match hero.loader {
                    Loader::Vanilla => color::Primary::TINT,
                    Loader::Fabric => color::Accents::VIOLET_TINT,
                    Loader::Forge => color::Accents::AMBER_TINT,
                    Loader::Quilt => color::Accents::CYAN_TINT,
                    Loader::NeoForge => color::Accents::PINK_TINT,
                },
            ),
            chip(format!("MC {}", hero.mc_version), color::Text::MUTED, rgb!(255, 255, 255, 6%)),
        ]
        .spacing(8);

        let mut meta = format!("{} mods", hero.mod_count);
        if let Some(played) = hero.last_played {
            meta.push_str(&format!(" · last played {}", relative_time(played)));
        } else {
            meta.push_str(" · never played");
        }

        Some(
            container(
                stack![
                    // The biome scene fills the banner behind everything — the
                    // hero variant clips the corners and bakes the left fade.
                    container(
                        svg(hero_handle(hero.biome, hero.art_seed))
                            .content_fit(iced::ContentFit::Cover)
                            .width(Fill)
                            .height(Fill)
                    )
                    .width(Fill)
                    .height(Fill),
                    container(
                        column![
                            mouse_area(
                                column![
                                    chips,
                                    Text::new(hero.name.clone())
                                        .size(34)
                                        .font(fonts::Inter::EXTRA_BOLD)
                                        .color(color::Text::HEADLINE),
                                    Text::new(meta).size(13).font(fonts::Inter::REGULAR).color(color::Text::DEFAULT),
                                ]
                                .spacing(14),
                            )
                            .on_press(Message::OpenInstance(hero.id.clone())),
                            row![
                                TextButton::content(
                                    row![
                                        icon(Icon::lucide().play(), 14.0, color::Primary::ON, 1.0),
                                        Text::new(if playing { "Resume Session" } else { "Play" })
                                            .size(13)
                                            .font(fonts::Inter::BOLD)
                                            .color(color::Primary::ON),
                                    ]
                                    .spacing(8)
                                    .align_y(alignment::Vertical::Center)
                                )
                                .variant(crate::app::components::button::Variant::Primary)
                                .padding(Padding::new(0.0).left(22).right(22).top(14).bottom(14))
                                .on_press(Message::Play(hero.name.clone())),
                                hero_menu_button(&hero.name, "folder", Icon::lucide().folder()),
                                hero_menu_button(&hero.name, "settings", Icon::lucide().settings()),
                                hero_menu_button(&hero.name, "more", Icon::lucide().ellipsis_vertical()),
                            ]
                            .spacing(10)
                            .align_y(alignment::Vertical::Center),
                        ]
                        .spacing(14)
                        .padding(Padding::new(0.0).left(28).right(28).bottom(28)),
                    )
                    .width(Fill)
                    .height(Fill)
                    .align_y(alignment::Vertical::Center),
                ]
                .width(Fill)
                .height(300),
            )
            .width(Fill)
            .height(300)
            .style(|_| container::Style {
                border: Border {
                    radius: 18.0.into(),
                    width: 1.0,
                    color: color::Surface::LINE,
                },
                ..container::Style::default()
            })
            .into(),
        )
    }

    /// The four quick-action cards — dummy entries until the features land.
    pub(super) fn quick_actions(&self) -> Element<'_, Message> {
        let cards: [(&'static str, Color, Color, &'static str, &'static str); 4] = [
            (
                Icon::lucide().plus(),
                color::Primary::DEFAULT,
                color::Primary::TINT,
                "New Instance",
                "From modpack, CurseForge, or scratch",
            ),
            (
                Icon::lucide().download(),
                color::Accents::CYAN,
                color::Accents::CYAN_TINT,
                "Import",
                ".zip, .mrpack, CurseForge",
            ),
            (
                Icon::lucide().server(),
                color::Accents::AMBER,
                color::Accents::AMBER_TINT,
                "Join Server",
                "Realms · hosted · friends",
            ),
            (
                Icon::lucide().users(),
                color::Accents::VIOLET_LIGHT,
                color::Accents::VIOLET_TINT,
                "Co-op Sync",
                "3 friends online now",
            ),
        ];
        row(cards
            .into_iter()
            .map(|(icon_markup, ink, fill, title, subtitle)| quick_action_card(icon_markup, ink, fill, title, subtitle, Message::QuickAction(title))))
        .spacing(20)
        .width(Fill)
        .into()
    }
}

/// A hero icon button — one of the instance's context actions.
fn hero_menu_button(instance: &str, action: &'static str, icon_markup: &'static str) -> Element<'static, Message> {
    IconButton::new(icon_markup)
        .width(44)
        .height(44)
        .icon_size(16.0)
        .on_press(Message::InstanceMenu {
            instance: instance.to_string(),
            action,
        })
        .into()
}

/// One quick-action card.
fn quick_action_card(
    icon_markup: &'static str,
    ink: Color,
    fill: Color,
    title: &'static str,
    subtitle: &'static str,
    on_press: Message,
) -> Element<'static, Message> {
    TextButton::content(
        row![
            container(icon(icon_markup, 18.0, ink, 1.0))
                .width(44)
                .height(44)
                .align_x(alignment::Horizontal::Center)
                .align_y(alignment::Vertical::Center)
                .style(move |_| container::Style {
                    background: Some(Background::Color(fill)),
                    border: Border {
                        radius: 10.0.into(),
                        ..Border::default()
                    },
                    ..container::Style::default()
                }),
            column![
                Text::new(title).size(13).font(fonts::Inter::SEMI_BOLD).color(color::Text::HEADLINE),
                Text::new(subtitle).size(11).font(fonts::Inter::REGULAR).color(color::Text::MUTED),
            ]
            .spacing(2),
        ]
        .spacing(12)
        .align_y(alignment::Vertical::Center)
        .width(Fill),
    )
    .padding(Padding::new(0.0).left(16).right(16).top(14).bottom(14))
    .width(Fill)
    .on_press(on_press)
    .into()
}

/// The status chip — the pulsing dot plus the uppercase label.
fn status_chip(state: InstanceState) -> Element<'static, Message> {
    let (label, ink, fill, pulsing) = match state {
        InstanceState::Playing => ("PLAYING", color::Primary::DEFAULT, color::Primary::TINT, true),
        InstanceState::Idle => ("IDLE", color::Text::MUTED, rgb!(255, 255, 255, 6%), false),
    };
    container(
        row![
            PulseDot::with_color(ink).size(6.0).pulsing(pulsing),
            Text::new(label).size(10).font(fonts::JetBrainsMono::SEMI_BOLD).color(ink),
        ]
        .spacing(6)
        .align_y(alignment::Vertical::Center),
    )
    .padding(Padding::new(0.0).left(8).right(8).top(3).bottom(3))
    .style(move |_| container::Style {
        background: Some(Background::Color(fill)),
        border: Border {
            radius: 999.0.into(),
            ..Border::default()
        },
        ..container::Style::default()
    })
    .into()
}

/// A coarse humanized "time ago" for the hero meta line.
fn relative_time(then: DateTime<Local>) -> String {
    let delta = Local::now() - then;
    let minutes = delta.num_minutes().max(0);
    if minutes < 1 {
        "just now".to_string()
    } else if minutes < 60 {
        format!("{minutes} minutes ago")
    } else if minutes < 60 * 24 {
        format!("{} hours ago", minutes / 60)
    } else {
        format!("{} days ago", minutes / (60 * 24))
    }
}
