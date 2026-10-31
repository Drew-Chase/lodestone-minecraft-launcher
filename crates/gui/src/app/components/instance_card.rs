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
use iced::widget::image::Handle;
use iced::widget::{Space, Text, column, container, mouse_area, row, stack};
use iced::{Background, Border, Color, Element, Fill, Length, Padding, alignment, color, mouse};

use crate::app::components::badge::{Badge, loader_palette};
use crate::app::components::{ProgressBar, TextButton};
use crate::app::downloads::{JobState, Pending};

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
/// The side of a compact row's square thumbnail.
pub const THUMB_SIZE: f32 = 44.0;
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
    /// A row of the context menu was picked. A separator sends `None`.
    pub menu: Box<dyn Fn(Option<instance_menu::Action>) -> Message>,
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

/// The Play button of an installed instance. While its install has not ended, a disabled button says
/// how far the install has got.
fn play_slot<Message: Clone + 'static>(on_press: Message, install: Option<&Pending>) -> Element<'static, Message> {
    match install {
        None => play_button(on_press),
        Some(pending) => TextButton::primary(install_status(pending))
            .padding(Padding::new(0.0).left(16).right(16).top(8).bottom(8))
            .disabled(true)
            .into(),
    }
}

/// What an install that has not ended shows: its stage and percent, or why it has not started.
pub fn install_status(pending: &Pending) -> String {
    let progress = pending.progress.as_ref();
    let fraction = progress.and_then(|progress| progress.fraction);
    match (&pending.state, progress, fraction) {
        (JobState::Queued, _, _) => lang::t("downloads.queued").to_owned(),
        (JobState::Paused, _, Some(fraction)) => lang::t_with("downloads.paused_at", &[("percent", percent(fraction).as_str())]),
        (JobState::Paused, _, None) => lang::t("downloads.paused").to_owned(),
        (_, Some(progress), Some(fraction)) => lang::t_with(
            "downloads.card_progress",
            &[("stage", progress.stage.label()), ("percent", percent(fraction).as_str())],
        ),
        (_, Some(progress), None) => progress.stage.label().to_owned(),
        _ => lang::t("downloads.starting").to_owned(),
    }
}

/// A fraction as a whole percentage, without the sign.
fn percent(fraction: f32) -> String {
    format!("{:.0}", (fraction * 100.0).min(100.0))
}

/// The scrim over an art box while its install has not ended: the stage and percent, with a bar under them.
fn install_overlay<Message: 'static>(pending: &Pending) -> Element<'static, Message> {
    let fraction = pending.progress.as_ref().and_then(|progress| progress.fraction);
    container(
        column![
            Text::new(install_status(pending))
                .size(12)
                .font(fonts::Inter::SEMI_BOLD)
                .color(color::Text::HEADLINE),
            ProgressBar::new(fraction.unwrap_or(0.0)).indeterminate(fraction.is_none()).width(Fill),
        ]
        .spacing(8)
        .width(Fill),
    )
    .padding(Padding::new(14.0))
    .width(Fill)
    .height(Fill)
    .align_y(alignment::Vertical::Bottom)
    .style(|_| container::Style {
        background: Some(Background::Color(Color { a: 0.55, ..color!(0x08090A) })),
        ..container::Style::default()
    })
    .into()
}

/// The art of a card or row: the instance's banner, with the install's scrim over it while it installs.
fn card_art<Message: 'static>(instance: &Instance, install: Option<&Pending>, banner: Option<Handle>) -> Element<'static, Message> {
    match install {
        Some(pending) => stack![instance_art::card_banner(instance, banner), install_overlay(pending)].into(),
        None => instance_art::card_banner(instance, banner),
    }
}

/// Wraps a card's or row's body in its context menu. The menu lists the instance's rows, or only Cancel
/// install while the install has not ended.
fn with_menu<Message: Clone + 'static>(
    body: impl Into<Element<'static, Message>>,
    instance: &Instance,
    installing: bool,
    on_select: Box<dyn Fn(Option<instance_menu::Action>) -> Message>,
) -> Element<'static, Message> {
    let menu = instance_menu::menu(installing, true);
    let actions = menu.actions;
    ContextMenu::new(body, instance.id.clone())
        .items(menu.items)
        .on_select(move |index| on_select(actions.get(index).copied().flatten()))
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
pub fn instance_card<Message: Clone + 'static>(
    instance: &Instance,
    hovered: bool,
    messages: InstanceMessages<Message>,
    install: Option<&Pending>,
    banner: Option<Handle>,
) -> Element<'static, Message> {
    let body = container(
        column![
            container(card_art(instance, install, banner))
                .padding(Padding::new(1.0).bottom(0.0))
                .width(Fill)
                .height(CARD_ART_HEIGHT),
            container(
                column![
                    row![
                        column![title_block(instance), version_line(instance)].spacing(2),
                        Space::new().width(Fill),
                        play_slot(messages.play, install),
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
    with_menu(grown, instance, install.is_some(), messages.menu)
}

/// The Compact row — the grid card's info without the art, one tight line. The whole row opens the
/// instance except for its Play button, and a right-click opens its context menu.
pub fn compact_row<Message: Clone + 'static>(
    instance: &Instance,
    messages: InstanceMessages<Message>,
    install: Option<&Pending>,
    thumb: Option<Handle>,
) -> Element<'static, Message> {
    let body = container(
        row![
            container(instance_art::thumbnail(instance, thumb)).width(THUMB_SIZE).height(THUMB_SIZE),
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
            play_slot(messages.play, install),
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

    with_menu(
        mouse_area(body).on_press(messages.open).interaction(mouse::Interaction::Pointer),
        instance,
        install.is_some(),
        messages.menu,
    )
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
pub fn table_row<Message: Clone + 'static>(
    index: usize,
    instance: &Instance,
    messages: InstanceMessages<Message>,
    install: Option<&Pending>,
) -> Element<'static, Message> {
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
            container(play_slot(messages.play, install)).width(Length::Fixed(COL_ACTIONS)),
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

    with_menu(
        mouse_area(body).on_press(messages.open).interaction(mouse::Interaction::Pointer),
        instance,
        install.is_some(),
        messages.menu,
    )
}
