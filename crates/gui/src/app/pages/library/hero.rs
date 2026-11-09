//! The Library hero banner for the most-recent instance, and the quick-action
//! cards beneath it.

use crate::app::components::art_cache::{ArtCache, ArtKey, Kind};
use crate::app::components::badge::{Badge, loader_palette};
use crate::app::components::hint::hint;
use crate::app::components::instance_art;
use crate::app::components::instance_card::status_chip;
use crate::app::components::page::{content_width, is_narrow, is_short};
use crate::app::components::scene_art::HERO_RADIUS;
use crate::app::components::{Dropdown, IconButton, TextButton};
use crate::app::lang;
use crate::app::preferences::HeroRule;
use crate::app::theme::{color, fonts};
use aurora_leaf::Icon;
use chrono::{DateTime, Local};
use iced::widget::{Text, column, container, mouse_area, row, stack};
use iced::{Background, Border, Color, Element, Fill, Padding, Size, alignment};
use iced_color_extension::rgb;

use super::instance::{Instance, InstanceState};
use super::instance_menu;
use super::{Library, Message, chip, icon};

/// The hero banner's height.
pub(super) const HERO_HEIGHT: f32 = 300.0;
/// The hero banner's height on a short window, where the list would otherwise start below the fold.
pub(super) const HERO_SHORT_HEIGHT: f32 = 230.0;

/// The hero banner's height in a window `viewport` big.
pub(super) fn hero_height(viewport: Size) -> f32 {
    if is_short(viewport.height) { HERO_SHORT_HEIGHT } else { HERO_HEIGHT }
}

impl Library {
    /// The instance the hero shows: the one the library's hero rule favours.
    pub(super) fn hero_instance(&self) -> Option<&Instance> {
        self.instances.iter().max_by_key(|instance| Self::hero_order(self.hero_rule, instance))
    }

    /// The order a hero rule puts the instances in: the one it favours sorts last. Each key is a pair, and its second part
    /// breaks ties. An instance that was never launched sorts below every launched one.
    fn hero_order(rule: HeroRule, instance: &Instance) -> (i64, i64) {
        let played = instance.last_played.map_or(i64::MIN, |at| at.timestamp_millis());
        let installed = instance.installed_at.timestamp_millis();
        let recency = instance.recency().timestamp_millis();
        match rule {
            HeroRule::LastPlayed => (played, installed),
            HeroRule::LastInstalled => (installed, 0),
            HeroRule::LastPlayedAndInstalled => (recency, 0),
            HeroRule::MostPlayed => (i64::from(instance.playtime_seconds), recency),
        }
    }

    /// The most-recent instance's wide banner card — rounded with a
    /// hairline border, scene art behind a left-dark readability gradient.
    /// `None` without instances, so the page reserves no room for it.
    pub(super) fn hero(&self, viewport: Size, art: &ArtCache) -> Option<Element<'_, Message>> {
        let hero = self.hero_instance()?;
        let playing = hero.state == InstanceState::Playing;
        let height = hero_height(viewport);
        // A short window also sets the title smaller, so the banner's column still fits its height.
        let title_size = if is_short(viewport.height) { 26 } else { 34 };
        let rendered = match hero.art_path {
            Some(_) => None,
            None => art.get(ArtKey::new(Kind::Hero, hero.biome, hero.art_seed, content_width(viewport.width), height)),
        };

        let (ink, fill) = loader_palette(hero.loader);
        let chips = row![
            status_chip(hero.state),
            Badge::new(
                match &hero.loader_version {
                    Some(version) => format!("{} {version}", hero.loader.label()),
                    None => hero.loader.label().to_string(),
                },
                ink,
                fill,
            )
            .compact()
            .with_loader(hero.loader),
            chip(format!("MC {}", hero.mc_version), color::text_muted(), rgb!(255, 255, 255, 6%)),
        ]
        .spacing(8);

        let played = match hero.last_played {
            Some(played) => lang::t_with("library.hero.last_played", &[("time", relative_time(played).as_str())]),
            None => lang::t("library.hero.never_played").to_owned(),
        };
        let mods = lang::t_count("instance.mods", hero.mod_count as usize);
        let meta = lang::t_with("library.hero.meta", &[("mods", mods.as_str()), ("played", played.as_str())]);

        Some(
            container(
                stack![
                    // The biome scene fills the banner behind everything — the
                    // hero variant clips the corners and bakes the left fade.
                    container(instance_art::hero_banner(hero, rendered)).width(Fill).height(Fill),
                    container(
                        column![
                            mouse_area(
                                column![
                                    chips,
                                    Text::new(hero.name.clone())
                                        .size(title_size)
                                        .font(fonts::Inter::EXTRA_BOLD)
                                        .color(color::text_headline()),
                                    Text::new(meta).size(13).font(fonts::Inter::REGULAR).color(color::text_default()),
                                ]
                                .spacing(14),
                            )
                            .on_press(Message::OpenInstance(hero.id.clone())),
                            hero_actions(&hero.id, playing),
                        ]
                        .spacing(14)
                        .padding(Padding::new(0.0).left(28).right(28).bottom(28)),
                    )
                    .width(Fill)
                    .height(Fill)
                    .align_y(alignment::Vertical::Center),
                ]
                .width(Fill)
                .height(height),
            )
            .width(Fill)
            .height(height)
            .style(|_| container::Style {
                border: Border {
                    radius: HERO_RADIUS.into(),
                    width: 1.0,
                    color: color::surface_line(),
                },
                ..container::Style::default()
            })
            .into(),
        )
    }

    /// The four quick-action cards — dummy entries until the features land. At a narrow window they become one row of
    /// icon tiles without their subtitles, so four fit beside each other and the list keeps its room.
    pub(super) fn quick_actions(&self, viewport: Size) -> Element<'_, Message> {
        // Each card's text comes from the catalogue; its action carries its own id.
        let cards: [(&'static str, Color, Color, &'static str, &'static str, Message); 4] = [
            (
                Icon::lucide().plus(),
                color::primary(),
                color::primary_tint(),
                lang::t("library.quick.new_instance.title"),
                lang::t("library.quick.new_instance.subtitle"),
                Message::OpenNewInstance,
            ),
            (
                Icon::lucide().download(),
                color::cyan(),
                color::cyan_tint(),
                lang::t("library.quick.import.title"),
                lang::t("library.quick.import.subtitle"),
                Message::OpenImport,
            ),
            (
                Icon::lucide().server(),
                color::amber(),
                color::amber_tint(),
                lang::t("library.quick.join_server.title"),
                lang::t("library.quick.join_server.subtitle"),
                Message::OpenJoinServer,
            ),
            (
                Icon::lucide().users(),
                color::violet_light(),
                color::violet_tint(),
                lang::t("library.quick.co_op.title"),
                lang::t("library.quick.co_op.subtitle"),
                Message::OpenCoopSync,
            ),
        ];
        if is_narrow(viewport.width) {
            return row(cards
                .into_iter()
                .map(|(icon_markup, ink, fill, title, _subtitle, on_press)| quick_action_tile(icon_markup, ink, fill, title, on_press)))
            .spacing(10)
            .width(Fill)
            .into();
        }
        row(cards
            .into_iter()
            .map(|(icon_markup, ink, fill, title, subtitle, on_press)| quick_action_card(icon_markup, ink, fill, title, subtitle, on_press)))
        .spacing(20)
        .width(Fill)
        .into()
    }
}

/// A hero icon button — one of the instance's actions.
fn hero_icon_button(icon_markup: &'static str, on_press: Message) -> Element<'static, Message> {
    IconButton::new(icon_markup)
        .width(44)
        .height(44)
        .icon_size(16.0)
        .on_press(on_press)
        .into()
}

/// The hero's action row: Resume (or Play), a button that starts another process of the instance while
/// it plays, the folder and settings buttons, and the instance menu.
fn hero_actions(instance: &str, playing: bool) -> Element<'static, Message> {
    let id = instance.to_owned();
    let resume = TextButton::content(
        row![
            icon(Icon::lucide().play(), 14.0, color::primary_on(), 1.0),
            Text::new(if playing {
                lang::t("library.hero.resume")
            } else {
                lang::t("common.play")
            })
            .size(13)
            .font(fonts::Inter::BOLD)
            .color(color::primary_on()),
        ]
        .spacing(8)
        .align_y(alignment::Vertical::Center),
    )
    .variant(crate::app::components::button::Variant::Primary)
    .padding(Padding::new(0.0).left(22).right(22).top(14).bottom(14))
    .on_press(Message::Play(id.clone()));

    let mut actions = row![resume];
    if playing {
        actions = actions.push(hint(
            hero_icon_button(Icon::lucide().copy(), Message::LaunchNew(id.clone())),
            lang::t("game.new_process"),
        ));
    }
    actions
        .push(hero_icon_button(Icon::lucide().folder(), Message::RevealInstance(id.clone())))
        .push(hero_icon_button(Icon::lucide().settings(), Message::EditInstance(id.clone())))
        .push(hero_menu(&id))
        .spacing(10)
        .align_y(alignment::Vertical::Center)
        .into()
}

/// The hero's ⋯ menu: the card's rows, without Reveal, which the folder button beside it already does.
fn hero_menu(instance: &str) -> Element<'static, Message> {
    let id = instance.to_owned();
    let menu = instance_menu::menu(false, false);
    let actions = menu.actions;
    Dropdown::new(Icon::lucide().ellipsis_vertical())
        .items(menu.items)
        .width(44)
        .height(44)
        .on_select(move |index| Message::InstanceMenu {
            instance: id.clone(),
            action: actions.get(index).copied().flatten(),
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
                Text::new(title).size(13).font(fonts::Inter::SEMI_BOLD).color(color::text_headline()),
                Text::new(subtitle).size(11).font(fonts::Inter::REGULAR).color(color::text_muted()),
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

/// One quick action at a narrow window: its icon and title. The title stays on one line, so a tile never grows taller.
fn quick_action_tile(icon_markup: &'static str, ink: Color, fill: Color, title: &'static str, on_press: Message) -> Element<'static, Message> {
    TextButton::content(
        row![
            container(icon(icon_markup, 16.0, ink, 1.0))
                .width(32)
                .height(32)
                .align_x(alignment::Horizontal::Center)
                .align_y(alignment::Vertical::Center)
                .style(move |_| container::Style {
                    background: Some(Background::Color(fill)),
                    border: Border {
                        radius: 8.0.into(),
                        ..Border::default()
                    },
                    ..container::Style::default()
                }),
            Text::new(title)
                .size(13)
                .font(fonts::Inter::SEMI_BOLD)
                .color(color::text_headline())
                .wrapping(iced::widget::text::Wrapping::None),
        ]
        .spacing(10)
        .align_y(alignment::Vertical::Center),
    )
    .padding(Padding::new(0.0).left(12).right(12).top(10).bottom(10))
    .width(Fill)
    .on_press(on_press)
    .into()
}

/// A coarse humanized "time ago" for the hero meta line.
fn relative_time(then: DateTime<Local>) -> String {
    let delta = Local::now() - then;
    let minutes = delta.num_minutes().max(0);
    if minutes < 1 {
        lang::t("time.just_now").to_owned()
    } else if minutes < 60 {
        lang::t_count("time.minutes_ago", minutes as usize)
    } else if minutes < 60 * 24 {
        lang::t_count("time.hours_ago", (minutes / 60) as usize)
    } else {
        lang::t_count("time.days_ago", (minutes / (60 * 24)) as usize)
    }
}
