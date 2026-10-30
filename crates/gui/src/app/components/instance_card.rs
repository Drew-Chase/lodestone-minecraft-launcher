//! Instance display components — the three Library views share these:
//! the full grid card ([`instance_card`]), the image-less [`compact_row`]
//! (Compact), and the [`table`] (Table) — plus the shared loader chip and
//! play button.

use crate::app::components::instance_art;
use crate::app::pages::library::instance::{Instance, Loader};
use crate::app::theme::color;
use crate::app::theme::fonts;
use iced::widget::{Space, Text, column, container, mouse_area, row};
use iced::{Background, Border, Color, Element, Fill, Length, Padding, alignment, color};

use crate::app::components::TextButton;
use crate::app::components::badge::{Badge, loader_palette};
use crate::app::lang;

/// The grid card's fixed height — art box plus info block. Fixed so the
/// virtualized rows keep an exact pitch.
pub const CARD_HEIGHT: f32 = 264.0;
/// The grid art box height (the top portion of [`CARD_HEIGHT`]).
pub const CARD_ART_HEIGHT: f32 = 180.0;
/// The compact row's fixed height.
pub const COMPACT_ROW_HEIGHT: f32 = 64.0;
/// One table row's fixed height (the header is not virtualized).
pub const TABLE_ROW_HEIGHT: f32 = 52.0;

/// The instance's banner, covering its box: its uploaded image, or the generated scene art.
fn scene_art<Message: 'static>(instance: &Instance) -> Element<'static, Message> {
    instance_art::card_banner(instance)
}

/// The instance's display name.
fn title_block(instance: &Instance) -> Text<'static> {
    Text::new(instance.name.clone())
        .size(15)
        .font(fonts::Inter::EXTRA_BOLD)
        .color(color::Text::HEADLINE)
}

/// The version · loader line under the name.
fn version_line(instance: &Instance) -> Text<'static> {
    Text::new(format!(
        "{} · {}{}",
        instance.mc_version,
        instance.loader.label(),
        instance.loader_version.as_ref().map(|version| format!(" {version}")).unwrap_or_default()
    ))
    .size(11)
    .font(fonts::JetBrainsMono::REGULAR)
    .color(color::Text::MUTED)
}

/// Whole hours of playtime, as the `{hours}h` unit.
fn playtime_hours(playtime_seconds: u32) -> String {
    let hours = (playtime_seconds / 3600).to_string();
    lang::t_with("common.hours", &[("hours", hours.as_str())])
}

/// The grid card's footer: playtime and the mod count.
fn stats_line(instance: &Instance) -> String {
    let hours = playtime_hours(instance.playtime_seconds);
    let mods = lang::t_count("instance.mods", instance.mod_count as usize);
    lang::t_with("instance.stats", &[("hours", hours.as_str()), ("mods", mods.as_str())])
}

/// The Compact row's line: playtime, loader, game version, and mod count.
fn compact_stats(instance: &Instance) -> String {
    let hours = playtime_hours(instance.playtime_seconds);
    let mods = lang::t_count("instance.mods", instance.mod_count as usize);
    lang::t_with(
        "instance.compact_stats",
        &[
            ("hours", hours.as_str()),
            ("loader", instance.loader.label()),
            ("version", instance.mc_version.as_str()),
            ("mods", mods.as_str()),
        ],
    )
}

/// The small green play button used by every view.
fn play_button<Message: Clone + 'static>(on_press: Message) -> Element<'static, Message> {
    TextButton::primary(lang::t("common.play"))
        .padding(Padding::new(0.0).left(16).right(16).top(8).bottom(8))
        .on_press(on_press)
        .into()
}

/// The loader chip — uppercase mono label on the loader's tinted fill.
pub fn loader_chip<Message: 'static>(loader: Loader) -> Element<'static, Message> {
    let (ink, fill) = loader_palette(loader);
    Badge::new(loader.label(), ink, fill).compact().with_loader(loader).into()
}

/// The card's dark panel style — shared by the grid card and compact row.
fn card_panel() -> container::Style {
    container::Style {
        background: Some(Background::Color(color!(0x0D0F12))),
        border: Border {
            radius: 14.0.into(),
            width: 1.0,
            color: color::Surface::LINE,
        },
        ..container::Style::default()
    }
}

/// The full grid card — art on top, title, version row, play button,
/// footer stats. The height is driven by the art box; keep it fixed so the
/// virtual rows stay uniform.
pub fn instance_card<Message: Clone + 'static>(instance: &Instance, on_play: Message, on_open: Message) -> Element<'static, Message> {
    container(
        column![
            container(scene_art(instance)).width(Fill).height(CARD_ART_HEIGHT),
            container(
                column![
                    row![
                        mouse_area(column![title_block(instance), version_line(instance)].spacing(2)).on_press(on_open),
                        Space::new().width(Fill),
                        play_button(on_play),
                    ]
                    .align_y(alignment::Vertical::Center),
                    row![
                        Text::new(stats_line(instance))
                            .size(11)
                            .font(fonts::Inter::REGULAR)
                            .color(color::Text::MUTED),
                        Space::new().width(Fill),
                        loader_chip(instance.loader),
                    ]
                    .spacing(8)
                    .align_y(alignment::Vertical::Center),
                ]
                .spacing(8)
                .padding(Padding::new(0.0).left(14).right(14).top(10).bottom(10)),
            )
            .width(Fill),
        ]
        .spacing(0),
    )
    .width(Fill)
    .height(Length::Fixed(CARD_HEIGHT))
    .style(|_| card_panel())
    .into()
}

/// The Compact row — the grid card's info without the art, one tight line.
pub fn compact_row<Message: Clone + 'static>(instance: &Instance, on_play: Message, on_open: Message) -> Element<'static, Message> {
    container(
        row![
            container(scene_art(instance)).width(44).height(44),
            mouse_area(
                column![
                    title_block(instance),
                    Text::new(compact_stats(instance))
                        .size(11)
                        .font(fonts::JetBrainsMono::REGULAR)
                        .color(color::Text::MUTED),
                ]
                .spacing(2),
            )
            .on_press(on_open),
            Space::new().width(Fill),
            loader_chip(instance.loader),
            play_button(on_play),
        ]
        .spacing(12)
        .align_y(alignment::Vertical::Center)
        .width(Fill),
    )
    .width(Fill)
    .height(Length::Fixed(COMPACT_ROW_HEIGHT))
    .align_y(alignment::Vertical::Center)
    .padding(Padding::new(0.0).left(12).right(12))
    .style(|_| card_panel())
    .into()
}

/// The table's column widths — shared by [`table_header`] and
/// [`table_row`] so the columns line up exactly.
const COL_VERSION: f32 = 110.0;
const COL_LOADER: f32 = 110.0;
const COL_MODS: f32 = 80.0;
const COL_PLAYTIME: f32 = 110.0;
const COL_ACTIONS: f32 = 90.0;

/// The table's header row.
pub fn table_header<Message: 'static>() -> Element<'static, Message> {
    let header = |label: &'static str, width: Length| {
        container(
            Text::new(label)
                .size(10)
                .font(fonts::JetBrainsMono::SEMI_BOLD)
                .color(color::Text::PLACEHOLDER),
        )
        .width(width)
    };
    row![
        header(lang::t("library.table.name"), Fill),
        header(lang::t("library.table.version"), Length::Fixed(COL_VERSION)),
        header(lang::t("library.table.loader"), Length::Fixed(COL_LOADER)),
        header(lang::t("library.table.mods"), Length::Fixed(COL_MODS)),
        header(lang::t("library.table.playtime"), Length::Fixed(COL_PLAYTIME)),
        header("", Length::Fixed(COL_ACTIONS)),
    ]
    .spacing(12)
    .padding(Padding::new(0.0).left(16).right(16).top(12).bottom(12))
    .into()
}

/// One striped table row. `index` drives the stripe parity within the
/// whole (unwindowed) list so stripes stay stable while scrolling.
pub fn table_row<Message: Clone + 'static>(index: usize, instance: &Instance, on_play: Message, on_open: Message) -> Element<'static, Message> {
    container(
        row![
            mouse_area(
                column![
                    title_block(instance),
                    Text::new(instance.directory_name.clone())
                        .size(10)
                        .font(fonts::JetBrainsMono::REGULAR)
                        .color(color::Text::PLACEHOLDER),
                ]
                .spacing(2)
                .width(Fill),
            )
            .on_press(on_open),
            Text::new(instance.mc_version.clone())
                .size(12)
                .color(color::Text::MUTED)
                .width(Length::Fixed(COL_VERSION)),
            container(loader_chip(instance.loader)).width(Length::Fixed(COL_LOADER)),
            Text::new(format!("{}", instance.mod_count))
                .size(12)
                .color(color::Text::MUTED)
                .width(Length::Fixed(COL_MODS)),
            Text::new(playtime_hours(instance.playtime_seconds))
                .size(12)
                .color(color::Text::MUTED)
                .width(Length::Fixed(COL_PLAYTIME)),
            container(play_button(on_play)).width(Length::Fixed(COL_ACTIONS)),
        ]
        .spacing(12)
        .align_y(alignment::Vertical::Center)
        .width(Fill),
    )
    .width(Fill)
    .height(Length::Fixed(TABLE_ROW_HEIGHT))
    .padding(Padding::new(0.0).left(16).right(16))
    .style(move |_| container::Style {
        background: Some(Background::Color(if index % 2 == 1 { color!(0x101216) } else { Color::TRANSPARENT })),
        ..container::Style::default()
    })
    .into()
}

/// The traditional table — header plus one row per instance, in one panel.
/// The Library's virtualized path composes [`table_header`] and
/// [`table_row`] directly; this covers the non-virtualized use.
#[allow(dead_code)]
pub fn table<Message: Clone + 'static>(
    instances: &[Instance],
    on_play: impl Fn(usize) -> Message,
    on_open: impl Fn(usize) -> Message,
) -> Element<'static, Message> {
    let mut rows = column![table_header()].spacing(0);
    for (index, instance) in instances.iter().enumerate() {
        rows = rows.push(table_row(index, instance, on_play(index), on_open(index)));
    }

    container(rows)
        .width(Fill)
        .style(|_| container::Style {
            background: Some(Background::Color(color!(0x0D0F12))),
            border: Border {
                radius: 12.0.into(),
                width: 1.0,
                color: color::Surface::LINE,
            },
            ..container::Style::default()
        })
        .into()
}
