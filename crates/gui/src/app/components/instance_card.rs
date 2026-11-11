//! Instance display components — the three Library views share these:
//! the full grid card ([`instance_card`]), the image-less [`compact_row`]
//! (Compact), and the [`table_row`] (Table) — plus the shared loader chip,
//! status chip and play button. Each one opens its instance from anywhere on
//! it except its buttons, and opens the instance's context menu on a
//! right-click.

use std::rc::Rc;

use crate::app::components::anim;
use crate::app::components::context_menu::ContextMenu;
use crate::app::components::hint::hint;
use crate::app::components::hover_scale::HoverScale;
use crate::app::components::hover_track::HoverTrack;
use crate::app::components::icon_button::ghost_button_style;
use crate::app::components::instance_art;
use crate::app::components::instance_icon;
use crate::app::lang;
use crate::app::pages::library::instance::{Instance, InstanceState, Loader};
use crate::app::pages::library::instance_menu;
use crate::app::theme::color;
use crate::app::theme::fonts;
use crate::app::theme::utility;
use aurora_leaf::Icon;
use iced::widget::image::Handle;
use iced::widget::{Row, Space, Text, column, container, mouse_area, row, stack};
use iced::{Background, Border, Color, Element, Fill, Gradient, Length, Padding, alignment, color, gradient, mouse};
use iced_color_extension::rgb;

use crate::app::components::badge::{Badge, loader_palette};
use crate::app::components::table::{self, Column};
use crate::app::components::{Dropdown, IconButton, ProgressBar, PulseDot, TextButton};
use crate::app::downloads::{JobState, Pending};

/// The grid card's art box height. The art sits one pixel inside the card's border, so its top edge is one
/// pixel down.
pub const CARD_ART_HEIGHT: f32 = 180.0;
/// The art's inset from the card's border, on its top, left and right.
const ART_INSET: f32 = 1.0;
/// The box the banner fills: the art's height less its inset at the top.
const ART_BOX_HEIGHT: f32 = CARD_ART_HEIGHT - ART_INSET;
/// The grid card's footer, under its one-pixel rule.
const FOOTER_HEIGHT: f32 = 43.0;
/// The grid card's fixed height: the art box, the rule and the footer. Fixed so the virtualized rows keep an
/// exact pitch.
pub const CARD_HEIGHT: f32 = CARD_ART_HEIGHT + 1.0 + FOOTER_HEIGHT;
/// The grid card's corner radius. The hover ring follows it.
const CARD_CORNER: f32 = 14.0;
/// The ⋯ button's side, on the art's top-right corner.
const KEBAB_SIZE: f32 = 32.0;
/// The side of the instance icon beside a grid card's title.
const CARD_ICON_SIZE: f32 = 36.0;
/// The compact row's fixed height.
pub const COMPACT_ROW_HEIGHT: f32 = 64.0;
/// The side of a compact row's square thumbnail.
pub const THUMB_SIZE: f32 = 44.0;
/// One table row's fixed height (the header is not virtualized).
pub const TABLE_ROW_HEIGHT: f32 = 52.0;
/// The side of the padlock on a compact row or a table row, in pixels.
const LIST_LOCK_SIZE: f32 = 28.0;

/// What one instance's card or row sends: the Play button, a press on its body, the pointer
/// entering and leaving a grid card, and the rows of its context menu.
pub struct InstanceMessages<Message> {
    /// The Play button.
    pub play: Message,
    /// A press anywhere on the body, except the buttons.
    pub open: Message,
    /// The padlock of a locked instance, which asks to unlock it.
    pub unlock: Message,
    /// The pointer entered a grid card. Compact and Table rows do not hover.
    pub enter: Message,
    /// The pointer left a grid card.
    pub exit: Message,
    /// A row of the context menu was picked. A separator sends `None`. The right-click menu and the ⋯
    /// button both send through this.
    pub menu: Rc<dyn Fn(Option<instance_menu::Action>) -> Message>,
}

/// The instance's display name.
fn title_block(instance: &Instance) -> Text<'static> {
    Text::new(instance.name.clone())
        .size(15)
        .font(fonts::Inter::EXTRA_BOLD)
        .color(color::text_headline())
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
    .color(color::text_muted())
}

/// Whole hours of playtime, as the `{hours}h` unit.
fn playtime_hours(playtime_seconds: u32) -> String {
    let hours = whole_hours(playtime_seconds);
    lang::t_with("common.hours", &[("hours", hours.as_str())])
}

/// Whole hours of playtime as a bare count. The `instance.compact_stats` string adds its own `h`, so
/// [`playtime_hours`] there would print it twice.
fn whole_hours(playtime_seconds: u32) -> String {
    (playtime_seconds / 3600).to_string()
}

/// The Compact row's line: playtime, loader, game version, and mod count.
fn compact_stats(instance: &Instance) -> String {
    let hours = whole_hours(instance.playtime_seconds);
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

/// The dim over an art box while its install has not ended.
fn install_scrim<Message: 'static>() -> Element<'static, Message> {
    container(Space::new().width(Fill).height(Fill))
        .width(Fill)
        .height(Fill)
        .style(|_| container::Style {
            background: Some(Background::Color(Color { a: 0.55, ..color!(0x08090A) })),
            ..container::Style::default()
        })
        .into()
}

/// The install's stage and percent over a bar, shown in the footer while the install has not ended.
fn install_progress<Message: 'static>(pending: &Pending) -> Element<'static, Message> {
    let fraction = pending.progress.as_ref().and_then(|progress| progress.fraction);
    column![
        Text::new(install_status(pending))
            .size(11)
            .font(fonts::Inter::SEMI_BOLD)
            .color(color::text_headline()),
        ProgressBar::new(fraction.unwrap_or(0.0)).indeterminate(fraction.is_none()).width(Fill),
    ]
    .spacing(4)
    .width(Fill)
    .into()
}

/// The art of a card: the instance's banner, dimmed while its install has not ended.
fn card_art<Message: 'static>(instance: &Instance, install: Option<&Pending>, banner: Option<Handle>) -> Element<'static, Message> {
    let art = instance_art::card_banner(instance, banner, ART_BOX_HEIGHT);
    match install {
        Some(_) => stack![art, install_scrim()].into(),
        None => art,
    }
}

/// The dark fade at the foot of the art, so the title and the Play button read over the picture.
pub fn art_shade<Message: 'static>() -> Element<'static, Message> {
    container(Space::new().width(Fill).height(Fill))
        .width(Fill)
        .height(Fill)
        .style(|_| container::Style {
            background: Some(Background::Gradient(Gradient::Linear(
                gradient::Linear::new(0.0)
                    .add_stop(0.0, rgb!(8, 9, 10, 88%))
                    .add_stop(0.6, rgb!(8, 9, 10, 0%)),
            ))),
            ..container::Style::default()
        })
        .into()
}

/// The status chip: a pulsing dot and an uppercase label on a tinted fill. The library hero and a playing card
/// share it.
pub fn status_chip<Message: 'static>(state: InstanceState) -> Element<'static, Message> {
    let (label, ink, fill, pulsing) = match state {
        InstanceState::Playing => (lang::t("library.status.playing"), color::primary(), color::primary_tint(), true),
        InstanceState::Idle => (lang::t("library.status.idle"), color::text_muted(), rgb!(255, 255, 255, 6%), false),
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

/// The card's ⋯ button: the instance's menu, which the right-click menu lists too.
fn menu_button<Message: Clone + 'static>(
    menu: &instance_menu::Menu,
    on_select: Rc<dyn Fn(Option<instance_menu::Action>) -> Message>,
) -> Element<'static, Message> {
    let actions = menu.actions.clone();
    Dropdown::new(Icon::lucide().ellipsis_vertical())
        .items(menu.items.clone())
        .width(KEBAB_SIZE)
        .height(KEBAB_SIZE)
        .on_select(move |index| on_select(actions.get(index).copied().flatten()))
        .into()
}

/// The padlock of a locked instance: a button that asks to unlock it. It is dimmed while the instance installs, since there is
/// nothing to unlock until the install has made the instance.
fn padlock<Message: Clone + 'static>(on_press: Message, installing: bool, size: f32) -> Element<'static, Message> {
    let button = IconButton::new(Icon::lucide().lock())
        .size(size)
        .radius(8.0)
        .icon_size(14.0)
        .style(ghost_button_style())
        .disabled(installing);
    if installing {
        return button.into();
    }
    hint(button.on_press(on_press), lang::t("instance.lock.hint"))
}

/// Wraps a card's or row's body in its context menu. The menu lists the instance's rows, or only Cancel
/// install while the install has not ended, and picking a row sends `on_select`'s message.
fn with_menu<Message: Clone + 'static>(
    body: impl Into<Element<'static, Message>>,
    instance: &Instance,
    menu: instance_menu::Menu,
    on_select: Rc<dyn Fn(Option<instance_menu::Action>) -> Message>,
) -> Element<'static, Message> {
    let instance_menu::Menu { items, actions } = menu;
    ContextMenu::new(body, instance.id.clone())
        .items(items)
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

/// A footer figure: a muted glyph and its label.
pub fn stat<Message: 'static>(glyph: &'static str, label: String) -> Element<'static, Message> {
    row![
        utility::icon(glyph, 12.0, Some((color::text_muted(), color::text_muted()))),
        Text::new(label).size(11).font(fonts::JetBrainsMono::REGULAR).color(color::text_muted()),
    ]
    .spacing(6)
    .align_y(alignment::Vertical::Center)
    .into()
}

/// The grid card's footer: the playtime and the mod count with their glyphs, or the install's stage and bar
/// while it runs, then the loader chip.
fn card_footer<Message: 'static>(instance: &Instance, install: Option<&Pending>) -> Element<'static, Message> {
    let figures: Element<'static, Message> = match install {
        Some(pending) => install_progress(pending),
        None => row![
            stat(Icon::lucide().clock(), playtime_hours(instance.playtime_seconds)),
            stat(Icon::lucide().package(), lang::t_count("instance.mods", instance.mod_count as usize)),
        ]
        .spacing(12)
        .align_y(alignment::Vertical::Center)
        .into(),
    };
    row![container(figures).width(Fill), loader_chip(instance.loader)]
        .align_y(alignment::Vertical::Center)
        .padding(Padding::new(0.0).left(14).right(14))
        .width(Fill)
        .height(FOOTER_HEIGHT)
        .into()
}

/// The full grid card — the art on top, with the PLAYING chip, the ⋯ button, the title and the Play button laid
/// over it, and a footer with the playtime, the mod count and the loader. The height is fixed so the virtual rows
/// stay uniform. A hovered card grows and lights its border. The whole card opens the instance except for its
/// buttons, and a right-click opens the same menu as the ⋯ button.
pub fn instance_card<Message: Clone + 'static>(
    instance: &Instance,
    hovered: bool,
    messages: InstanceMessages<Message>,
    install: Option<&Pending>,
    banner: Option<Handle>,
) -> Element<'static, Message> {
    let menu = instance_menu::menu(install.is_some(), true, instance.locked);
    let kebab = menu_button(&menu, Rc::clone(&messages.menu));
    let lock: Element<'static, Message> = if instance.locked {
        padlock(messages.unlock.clone(), install.is_some(), KEBAB_SIZE)
    } else {
        Space::new().into()
    };
    let status: Element<'static, Message> = if instance.state == InstanceState::Playing {
        status_chip(instance.state)
    } else {
        Space::new().into()
    };
    let art = container(stack![
        card_art(instance, install, banner),
        art_shade(),
        column![
            row![status, Space::new().width(Fill), lock, kebab].align_y(alignment::Vertical::Top),
            Space::new().height(Fill),
            row![
                row![
                    instance_icon::icon(&instance.icon, CARD_ICON_SIZE),
                    column![title_block(instance), version_line(instance)].spacing(2),
                ]
                .spacing(12)
                .align_y(alignment::Vertical::Center),
                Space::new().width(Fill),
                play_slot(messages.play, install),
            ]
            .align_y(alignment::Vertical::Center),
        ]
        .padding(Padding::new(12.0))
        .width(Fill)
        .height(Fill),
    ])
    .padding(Padding::new(ART_INSET).bottom(0.0))
    .width(Fill)
    .height(CARD_ART_HEIGHT);

    let body = container(column![
        art,
        container(Space::new().width(Fill).height(1)).style(|_| container::Style {
            background: Some(Background::Color(color::surface_line())),
            ..container::Style::default()
        }),
        card_footer(instance, install),
    ])
    .width(Fill)
    .height(Length::Fixed(CARD_HEIGHT))
    .style(|_| card_panel(color::surface_line()));

    // The hover grows the press area, so hit-testing follows the grown card. The menu wraps the
    // growth, so the menu itself does not grow.
    let pressable = HoverTrack::new(
        mouse_area(body).on_press(messages.open).interaction(mouse::Interaction::Pointer),
        hovered,
        messages.enter,
        messages.exit,
    );
    let grown = HoverScale::new(pressable, hovered).ring(CARD_CORNER, anim::fade(color::primary(), 0.6));
    with_menu(grown, instance, menu, messages.menu)
}

/// The Compact row — the grid card's info without the art, one tight line. The whole row opens the
/// instance except for its Play button, and a right-click opens its context menu.
pub fn compact_row<Message: Clone + 'static>(
    instance: &Instance,
    messages: InstanceMessages<Message>,
    install: Option<&Pending>,
    thumb: Option<Handle>,
) -> Element<'static, Message> {
    // A locked instance wears its padlock beside the loader chip, so the trailing group is built with it when it is locked.
    let mut trailing = row![].spacing(12).align_y(alignment::Vertical::Center);
    if instance.locked {
        trailing = trailing.push(padlock(messages.unlock.clone(), install.is_some(), LIST_LOCK_SIZE));
    }
    let trailing = trailing.push(loader_chip(instance.loader)).push(play_slot(messages.play, install));
    let body = container(
        row![
            container(instance_art::thumbnail(instance, thumb)).width(THUMB_SIZE).height(THUMB_SIZE),
            column![
                title_block(instance),
                Text::new(compact_stats(instance))
                    .size(11)
                    .font(fonts::JetBrainsMono::REGULAR)
                    .color(color::text_muted()),
            ]
            .spacing(2),
            Space::new().width(Fill),
            trailing,
        ]
        .spacing(12)
        .align_y(alignment::Vertical::Center)
        .width(Fill),
    )
    .width(Fill)
    .height(Length::Fixed(COMPACT_ROW_HEIGHT))
    .align_y(alignment::Vertical::Center)
    .padding(Padding::new(0.0).left(12).right(12))
    .style(|_| card_panel(color::surface_line()));

    with_menu(
        mouse_area(body).on_press(messages.open).interaction(mouse::Interaction::Pointer),
        instance,
        instance_menu::menu(install.is_some(), true, instance.locked),
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
/// The gap between a table's cells, and the row's padding either side.
const TABLE_GAP: f32 = 12.0;
const TABLE_PAD: f32 = 16.0;

/// Which of the library table's optional columns show. Playtime and the actions always do; the others hide as the window
/// narrows, in the order [`table_columns`] gives.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct TableColumns {
    pub version: bool,
    pub loader: bool,
    pub mods: bool,
}

/// The columns that show in a table in a list `list_width` pixels wide. The loader hides first, then the mod count, then the
/// version.
pub fn table_columns(list_width: f32) -> TableColumns {
    const COLUMNS: [Column; 5] = [
        Column {
            width: COL_VERSION,
            hide: Some(2),
        },
        Column {
            width: COL_LOADER,
            hide: Some(0),
        },
        Column {
            width: COL_MODS,
            hide: Some(1),
        },
        Column {
            width: COL_PLAYTIME,
            hide: None,
        },
        Column {
            width: COL_ACTIONS,
            hide: None,
        },
    ];
    let available = list_width - 2.0 * TABLE_PAD;
    let shown = table::fit(&COLUMNS, available, table::NAME_MIN, TABLE_GAP);
    TableColumns {
        version: shown[0],
        loader: shown[1],
        mods: shown[2],
    }
}

/// The name cell's second line: the folder, then the facts of any column the row no longer shows.
fn name_detail(instance: &Instance, shown: TableColumns) -> String {
    let mut parts = vec![instance.directory_name.clone()];
    if !shown.version {
        parts.push(instance.mc_version.clone());
    }
    if !shown.loader {
        parts.push(instance.loader.label().to_string());
    }
    if !shown.mods {
        parts.push(lang::t_count("instance.mods", instance.mod_count as usize));
    }
    parts.join("  ·  ")
}

/// The table's header row, with the columns `shown` gives.
pub fn table_header<Message: 'static>(shown: TableColumns) -> Element<'static, Message> {
    let header = |label: &'static str, width: Length| {
        container(
            Text::new(label)
                .size(10)
                .font(fonts::JetBrainsMono::SEMI_BOLD)
                .color(color::text_placeholder())
                .wrapping(iced::widget::text::Wrapping::None),
        )
        .width(width)
    };
    let mut cells: Vec<Element<'static, Message>> = vec![header(lang::t("library.table.name"), Fill).into()];
    if shown.version {
        cells.push(header(lang::t("library.table.version"), Length::Fixed(COL_VERSION)).into());
    }
    if shown.loader {
        cells.push(header(lang::t("library.table.loader"), Length::Fixed(COL_LOADER)).into());
    }
    if shown.mods {
        cells.push(header(lang::t("library.table.mods"), Length::Fixed(COL_MODS)).into());
    }
    cells.push(header(lang::t("library.table.playtime"), Length::Fixed(COL_PLAYTIME)).into());
    cells.push(header("", Length::Fixed(COL_ACTIONS)).into());
    Row::with_children(cells)
        .spacing(TABLE_GAP)
        .padding(Padding::new(0.0).left(TABLE_PAD).right(TABLE_PAD).top(12).bottom(12))
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
    shown: TableColumns,
) -> Element<'static, Message> {
    // The name keeps one line and is clipped to its cell, so a long name never runs into the columns beside it.
    let details = column![
        Text::new(instance.name.clone())
            .size(15)
            .font(fonts::Inter::EXTRA_BOLD)
            .color(color::text_headline())
            .wrapping(iced::widget::text::Wrapping::None),
        Text::new(name_detail(instance, shown))
            .size(10)
            .font(fonts::JetBrainsMono::REGULAR)
            .color(color::text_placeholder())
            .wrapping(iced::widget::text::Wrapping::None),
    ]
    .spacing(2);
    // A locked instance wears its padlock before the name.
    let text: Element<'static, Message> = if instance.locked {
        row![padlock(messages.unlock.clone(), install.is_some(), LIST_LOCK_SIZE), details]
            .spacing(10)
            .align_y(alignment::Vertical::Center)
            .into()
    } else {
        details.into()
    };
    let name = container(text).width(Fill).clip(true);
    let mut cells: Vec<Element<'static, Message>> = vec![name.into()];
    if shown.version {
        cells.push(
            Text::new(instance.mc_version.clone())
                .size(12)
                .color(color::text_muted())
                .width(Length::Fixed(COL_VERSION))
                .into(),
        );
    }
    if shown.loader {
        cells.push(container(loader_chip(instance.loader)).width(Length::Fixed(COL_LOADER)).into());
    }
    if shown.mods {
        cells.push(
            Text::new(format!("{}", instance.mod_count))
                .size(12)
                .color(color::text_muted())
                .width(Length::Fixed(COL_MODS))
                .into(),
        );
    }
    cells.push(
        Text::new(playtime_hours(instance.playtime_seconds))
            .size(12)
            .color(color::text_muted())
            .width(Length::Fixed(COL_PLAYTIME))
            .into(),
    );
    cells.push(container(play_slot(messages.play, install)).width(Length::Fixed(COL_ACTIONS)).into());
    let body = container(
        Row::with_children(cells)
            .spacing(TABLE_GAP)
            .align_y(alignment::Vertical::Center)
            .width(Fill),
    )
    .width(Fill)
    .height(Length::Fixed(TABLE_ROW_HEIGHT))
    .padding(Padding::new(0.0).left(TABLE_PAD).right(TABLE_PAD))
    .style(move |_| container::Style {
        background: Some(Background::Color(if index % 2 == 1 { color!(0x101216) } else { Color::TRANSPARENT })),
        ..container::Style::default()
    });

    with_menu(
        mouse_area(body).on_press(messages.open).interaction(mouse::Interaction::Pointer),
        instance,
        instance_menu::menu(install.is_some(), true, instance.locked),
        messages.menu,
    )
}

#[cfg(test)]
#[path = "../../../tests/app/components/instance_card.rs"]
mod tests;
