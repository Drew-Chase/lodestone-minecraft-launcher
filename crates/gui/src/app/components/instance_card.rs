//! Instance display components — the three Library views share these:
//! the full grid card ([`instance_card`]), the image-less [`compact_row`]
//! (Compact), and the [`table_row`] (Table) — plus the shared loader chip and
//! play button. Each one opens its instance from anywhere on it except the
//! Play button, and opens the instance's context menu on a right-click.

use crate::app::components::anim;
use crate::app::components::context_menu::ContextMenu;
use crate::app::components::hover_scale::HoverScale;
use crate::app::components::instance_art;
use crate::app::lang;
use crate::app::pages::library::instance::{Instance, Loader};
use crate::app::pages::library::instance_menu;
use crate::app::theme::color;
use crate::app::theme::fonts;
use iced::widget::{Space, Text, column, container, mouse_area, row};
use iced::{Background, Border, Color, Element, Fill, Length, Padding, alignment, color, mouse};

use crate::app::components::TextButton;
use crate::app::components::badge::{Badge, loader_palette};

/// The grid card's fixed height — art box plus info block. Fixed so the
/// virtualized rows keep an exact pitch.
pub const CARD_HEIGHT: f32 = 264.0;
/// The grid art box height (the top portion of [`CARD_HEIGHT`]). The art sits one pixel inside
/// the card's border, so its top edge is one pixel down.
pub const CARD_ART_HEIGHT: f32 = 180.0;
/// The grid card's corner radius. The hover ring follows it.
const CARD_CORNER: f32 = 14.0;
/// The compact row's fixed height.
pub const COMPACT_ROW_HEIGHT: f32 = 64.0;
/// One table row's fixed height (the header is not virtualized).
pub const TABLE_ROW_HEIGHT: f32 = 52.0;

/// What one instance's card or row sends: the Play button, a press on its body, the pointer
/// entering and leaving a grid card, and the rows of its context menu.
pub struct InstanceMessages<Message> {
    /// The Play button.
    pub play: Message,
    /// A press anywhere on the body, except the Play button.
    pub open: Message,
    /// The pointer entered a grid card. Compact and Table rows do not hover.
    pub enter: Message,
    /// The pointer left a grid card.
    pub exit: Message,
    /// A row of the context menu was picked: the menu's index for it.
    pub menu: Box<dyn Fn(usize) -> Message>,
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

/// The card's dark panel, with its border colour — shared by the grid card and the compact row.
fn card_panel(border: Color) -> container::Style {
    container::Style {
        background: Some(Background::Color(color!(0x0D0F12))),
        border: Border {
            radius: CARD_CORNER.into(),
            width: 1.0,
            color: border,
        },
        ..container::Style::default()
    }
}

/// The full grid card — art on top, title, version row, play button, footer stats. The height is
/// driven by the art box; keep it fixed so the virtual rows stay uniform. A hovered card grows and
/// lights its border. The whole card opens the instance except for its Play button, and a
/// right-click opens its context menu.
pub fn instance_card<Message: Clone + 'static>(instance: &Instance, hovered: bool, messages: InstanceMessages<Message>) -> Element<'static, Message> {
    let body = container(
        column![
            container(instance_art::card_banner(instance))
                .padding(Padding::new(1.0).bottom(0.0))
                .width(Fill)
                .height(CARD_ART_HEIGHT),
            container(
                column![
                    row![
                        column![title_block(instance), version_line(instance)].spacing(2),
                        Space::new().width(Fill),
                        play_button(messages.play),
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
    .style(|_| card_panel(color::Surface::LINE));

    // The hover grows the press area, so hit-testing follows the grown card. The menu wraps the
    // growth, so the menu itself does not grow.
    let pressable = mouse_area(body)
        .on_press(messages.open)
        .on_enter(messages.enter)
        .on_exit(messages.exit)
        .interaction(mouse::Interaction::Pointer);
    let grown = HoverScale::new(pressable, hovered).ring(CARD_CORNER, anim::fade(color::Primary::DEFAULT, 0.6));
    ContextMenu::new(grown, instance.id.clone())
        .items(instance_menu::items())
        .on_select(messages.menu)
        .into()
}

/// The Compact row — the grid card's info without the art, one tight line. The whole row opens the
/// instance except for its Play button, and a right-click opens its context menu.
pub fn compact_row<Message: Clone + 'static>(instance: &Instance, messages: InstanceMessages<Message>) -> Element<'static, Message> {
    let body = container(
        row![
            container(instance_art::thumbnail(instance)).width(44).height(44),
            column![
                title_block(instance),
                Text::new(compact_stats(instance))
                    .size(11)
                    .font(fonts::JetBrainsMono::REGULAR)
                    .color(color::Text::MUTED),
            ]
            .spacing(2),
            Space::new().width(Fill),
            loader_chip(instance.loader),
            play_button(messages.play),
        ]
        .spacing(12)
        .align_y(alignment::Vertical::Center)
        .width(Fill),
    )
    .width(Fill)
    .height(Length::Fixed(COMPACT_ROW_HEIGHT))
    .align_y(alignment::Vertical::Center)
    .padding(Padding::new(0.0).left(12).right(12))
    .style(|_| card_panel(color::Surface::LINE));

    ContextMenu::new(
        mouse_area(body).on_press(messages.open).interaction(mouse::Interaction::Pointer),
        instance.id.clone(),
    )
    .items(instance_menu::items())
    .on_select(messages.menu)
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
/// whole (unwindowed) list so stripes stay stable while scrolling. The whole row opens the
/// instance except for its Play button, and a right-click opens its context menu.
pub fn table_row<Message: Clone + 'static>(index: usize, instance: &Instance, messages: InstanceMessages<Message>) -> Element<'static, Message> {
    let body = container(
        row![
            column![
                title_block(instance),
                Text::new(instance.directory_name.clone())
                    .size(10)
                    .font(fonts::JetBrainsMono::REGULAR)
                    .color(color::Text::PLACEHOLDER),
            ]
            .spacing(2)
            .width(Fill),
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
            container(play_button(messages.play)).width(Length::Fixed(COL_ACTIONS)),
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
    });

    ContextMenu::new(
        mouse_area(body).on_press(messages.open).interaction(mouse::Interaction::Pointer),
        instance.id.clone(),
    )
    .items(instance_menu::items())
    .on_select(messages.menu)
    .into()
}
