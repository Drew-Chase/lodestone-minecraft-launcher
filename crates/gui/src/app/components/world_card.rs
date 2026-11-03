//! World display components — the Worlds page and the instance Worlds tab share these: the grid card
//! ([`world_card`]) and the table's header and row ([`table_header`], [`table_row`]). Each one opens its menu
//! on a right-click, as the library's instance cards do.

use std::rc::Rc;

use aurora_leaf::Icon;
use iced::widget::image::{FilterMethod, Handle};
use iced::widget::{Row, Space, Text, column, container, image, mouse_area, row, stack};
use iced::{Background, Border, Color, ContentFit, Element, Fill, Length, Padding, alignment, color, mouse};
use iced_color_extension::rgb;

use crate::app::components::anim;
use crate::app::components::badge::Badge;
use crate::app::components::context_menu::ContextMenu;
use crate::app::components::hover_scale::HoverScale;
use crate::app::components::icon_button::secondary_button_style;
use crate::app::components::instance_card::{art_shade, stat};
use crate::app::components::screenshot_tile::placeholder_art;
use crate::app::components::{Dropdown, IconButton, TextButton};
use crate::app::lang;
use crate::app::pages::instance::files::{Difficulty, GameMode, WorldFile};
use crate::app::pages::instance::format::{format_bytes, format_playtime};
use crate::app::pages::instance::world_menu::{self, Action, Menu};
use crate::app::theme::{color, fonts, utility};

/// The grid card's banner height, per the reference.
pub const BANNER_HEIGHT: f32 = 150.0;
/// The footer under the banner's rule: the mode line and the seed row.
const FOOTER_HEIGHT: f32 = 80.0;
/// The grid card's fixed height, so the virtualized rows keep an exact pitch.
pub const CARD_HEIGHT: f32 = BANNER_HEIGHT + 1.0 + FOOTER_HEIGHT;
/// The widest a grid card is drawn: the reference's card width, so a card reads the same on the Worlds page and on
/// an instance's Worlds tab. A caller that gives a card more room keeps it at this width.
pub const CARD_MAX_WIDTH: f32 = 374.0;
/// The card's corner radius. The hover ring follows it.
const CARD_CORNER: f32 = 14.0;
/// The side of the icon on the banner: twice its 64px, so every pixel of the icon stays whole.
const BANNER_ICON: f32 = 128.0;
/// The side of the buttons in the seed row and the ⋯ button.
pub const BUTTON_SIZE: f32 = 30.0;
/// The side of a table row's thumbnail.
pub const THUMB_SIZE: f32 = 44.0;
/// One table row's fixed height.
pub const TABLE_ROW_HEIGHT: f32 = 64.0;

/// The table's column widths, shared by [`table_header`] and [`table_row`] so the columns line up.
const COL_MODE: f32 = 170.0;
const COL_SEED: f32 = 150.0;
const COL_STATS: f32 = 130.0;
const COL_LAST: f32 = 110.0;
const COL_ACTIONS: f32 = 130.0;

/// The red of a hardcore world's dot.
const HARDCORE: Color = color!(0xF04A4A);

/// What one world's card or row sends: its Load button, the rows of its menu, and the pointer entering and leaving a
/// grid card.
pub struct WorldMessages<Message> {
    /// The Load button.
    pub play: Message,
    /// A row of the world's menu was picked. A separator sends `None`. The ⋯ button, the copy button and the
    /// right-click menu all send through this.
    pub menu: Rc<dyn Fn(Option<Action>) -> Message>,
    /// The pointer entered a grid card.
    pub enter: Message,
    /// The pointer left a grid card.
    pub exit: Message,
}

/// What a card or row shows beside the save itself: the line under the name, and whether the world is pinned.
pub struct WorldLabels {
    pub subtitle: String,
    pub pinned: bool,
}

/// The dot and label colour a game mode is drawn in.
fn mode_ink(mode: GameMode) -> Color {
    match mode {
        GameMode::Survival => color::primary(),
        GameMode::Creative => color::cyan(),
        GameMode::Adventure => color::violet_light(),
        GameMode::Hardcore => HARDCORE,
    }
}

/// The card's dark panel and its border.
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

/// A grid card: the banner with the name over it, and a footer with the mode, the playtime and size, the seed,
/// and the buttons. The card grows while the pointer is over it. A right-click opens the same menu as ⋯.
pub fn world_card<Message: Clone + 'static>(
    world: &WorldFile,
    labels: WorldLabels,
    hovered: bool,
    messages: WorldMessages<Message>,
) -> Element<'static, Message> {
    let menu = world_menu::menu(world.seed.is_some(), labels.pinned);
    let body = container(column![
        banner(world, &labels),
        container(Space::new().width(Fill).height(1)).style(|_| container::Style {
            background: Some(Background::Color(color::surface_line())),
            ..container::Style::default()
        }),
        footer(world, &menu, Rc::clone(&messages.menu), messages.play),
    ])
    .width(Fill)
    .height(Length::Fixed(CARD_HEIGHT))
    .style(|_| card_panel(color::surface_line()));

    // The hover grows the press area, so hit-testing follows the grown card. The menu wraps the growth, so the
    // menu itself does not grow.
    let pressable = mouse_area(body)
        .on_enter(messages.enter)
        .on_exit(messages.exit)
        .interaction(mouse::Interaction::Pointer);
    let grown = HoverScale::new(pressable, hovered).ring(CARD_CORNER, anim::fade(color::primary(), 0.6));
    context(grown, world, menu, messages.menu)
}

/// The table's header row.
pub fn table_header<Message: 'static>() -> Element<'static, Message> {
    let header = |label: &'static str, width: Length| {
        container(
            Text::new(label)
                .size(10)
                .font(fonts::JetBrainsMono::SEMI_BOLD)
                .color(color::text_placeholder()),
        )
        .width(width)
    };
    row![
        Space::new().width(THUMB_SIZE),
        header(lang::t("worlds.table.name"), Fill),
        header(lang::t("worlds.table.mode"), Length::Fixed(COL_MODE)),
        header(lang::t("worlds.table.seed"), Length::Fixed(COL_SEED)),
        header(lang::t("worlds.table.stats"), Length::Fixed(COL_STATS)),
        header(lang::t("worlds.table.last_played"), Length::Fixed(COL_LAST)),
        header("", Length::Fixed(COL_ACTIONS)),
    ]
    .spacing(12)
    .padding(Padding::new(0.0).left(16).right(16).top(12).bottom(12))
    .into()
}

/// One striped table row: the thumbnail, the name with its pin, the mode, the seed and version, the playtime and
/// size, when it was last played, and the buttons. `index` drives the stripe, so it holds while the rows scroll.
pub fn table_row<Message: Clone + 'static>(
    index: usize,
    world: &WorldFile,
    labels: WorldLabels,
    last_played: String,
    messages: WorldMessages<Message>,
) -> Element<'static, Message> {
    let menu = world_menu::menu(world.seed.is_some(), labels.pinned);
    let title = Text::new(world.name.clone())
        .size(14)
        .font(fonts::Inter::SEMI_BOLD)
        .color(color::text_headline());
    let mut name_line: Row<'static, Message> = Row::new().spacing(6).align_y(alignment::Vertical::Center).push(title);
    if labels.pinned {
        name_line = name_line.push(utility::icon(Icon::lucide().pin(), 11.0, Some((color::primary(), color::primary()))));
    }
    if world.backed_up {
        name_line = name_line.push(backed_up_badge());
    }
    let body = container(
        row![
            thumbnail(world.icon.as_ref()),
            column![
                name_line,
                Text::new(labels.subtitle)
                    .size(10)
                    .font(fonts::JetBrainsMono::REGULAR)
                    .color(color::text_placeholder()),
            ]
            .spacing(2)
            .width(Fill),
            container(mode_line(world)).width(Length::Fixed(COL_MODE)),
            container(seed_and_version(world)).width(Length::Fixed(COL_SEED)),
            container(stats_line(world)).width(Length::Fixed(COL_STATS)),
            Text::new(last_played).size(12).color(color::text_muted()).width(Length::Fixed(COL_LAST)),
            container(
                row![kebab(&menu, Rc::clone(&messages.menu)), load_button(messages.play),]
                    .spacing(6)
                    .align_y(alignment::Vertical::Center),
            )
            .width(Length::Fixed(COL_ACTIONS)),
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

    context(mouse_area(body).interaction(mouse::Interaction::Pointer), world, menu, messages.menu)
}

/// The banner: the icon blurred behind, the icon at its own pixels on the right, the dark fade, the badges across
/// the top, and the name and its line over the foot.
fn banner<Message: 'static>(world: &WorldFile, labels: &WorldLabels) -> Element<'static, Message> {
    let backdrop: Element<'static, Message> = match &world.backdrop {
        Some(handle) => image(handle.clone())
            .content_fit(ContentFit::Cover)
            .border_radius(CARD_CORNER)
            .width(Fill)
            .height(Fill)
            .into(),
        None => placeholder_art(CARD_CORNER),
    };
    let picture: Element<'static, Message> = match &world.icon {
        Some(handle) => image(handle.clone())
            .filter_method(FilterMethod::Nearest)
            .width(BANNER_ICON)
            .height(BANNER_ICON)
            .into(),
        None => Space::new().into(),
    };
    container(stack![
        backdrop,
        container(picture)
            .width(Fill)
            .height(Fill)
            .align_x(alignment::Horizontal::Right)
            .align_y(alignment::Vertical::Center)
            .padding(Padding::new(0.0).right(24.0)),
        art_shade(),
        column![badges(world, labels.pinned), Space::new().height(Fill), name_block(world, labels)]
            .padding(Padding::new(12.0))
            .width(Fill)
            .height(Fill),
    ])
    .width(Fill)
    .height(Length::Fixed(BANNER_HEIGHT))
    .into()
}

/// The PINNED badge on the left and the BACKED UP badge on the right, along the banner's top.
fn badges<Message: 'static>(world: &WorldFile, pinned: bool) -> Element<'static, Message> {
    let mut items: Vec<Element<'static, Message>> = Vec::new();
    if pinned {
        items.push(
            Badge::new(lang::t("worlds.badge.pinned"), color::primary(), color::primary_tint())
                .compact()
                .into(),
        );
    }
    items.push(Space::new().width(Fill).into());
    if world.backed_up {
        items.push(backed_up_badge());
    }
    row(items).align_y(alignment::Vertical::Center).width(Fill).into()
}

/// The BACKED UP badge, shown while a zip of the save is in the instance's `backups/`.
pub fn backed_up_badge<Message: 'static>() -> Element<'static, Message> {
    Badge::new(lang::t("worlds.badge.backed_up"), color::cyan(), color::cyan_tint())
        .compact()
        .into()
}

/// The name over the banner's foot, with its subtitle beneath.
fn name_block<Message: 'static>(world: &WorldFile, labels: &WorldLabels) -> Element<'static, Message> {
    column![
        Text::new(world.name.clone())
            .size(15)
            .font(fonts::Inter::EXTRA_BOLD)
            .color(color::text_headline()),
        Text::new(labels.subtitle.clone())
            .size(11)
            .font(fonts::JetBrainsMono::REGULAR)
            .color(color::text_muted()),
    ]
    .spacing(2)
    .into()
}

/// The footer: the mode line and the stats on the first row, the seed field and the buttons on the second.
fn footer<Message: Clone + 'static>(
    world: &WorldFile,
    menu: &Menu,
    on_select: Rc<dyn Fn(Option<Action>) -> Message>,
    play: Message,
) -> Element<'static, Message> {
    let copy = IconButton::new(Icon::lucide().copy())
        .size(BUTTON_SIZE)
        .radius(9.0)
        .icon_size(13.0)
        .style(secondary_button_style())
        .disabled(world.seed.is_none())
        .on_press(on_select(Some(Action::CopySeed)));
    column![
        row![mode_line(world), Space::new().width(Fill), stats_line(world)].align_y(alignment::Vertical::Center),
        row![seed_field(world), copy, kebab(menu, Rc::clone(&on_select)), load_button(play),]
            .spacing(6)
            .align_y(alignment::Vertical::Center),
    ]
    .spacing(10)
    .padding(Padding::new(0.0).top(12.0).bottom(12.0).left(14.0).right(14.0))
    .width(Fill)
    .height(FOOTER_HEIGHT)
    .into()
}

/// The dot, the mode's name, and the difficulty after it, on one mono line.
fn mode_line<Message: 'static>(world: &WorldFile) -> Element<'static, Message> {
    let mut line: Row<'static, Message> = Row::new().spacing(6).align_y(alignment::Vertical::Center);
    if let Some(mode) = world.game_mode {
        line = line.push(dot(mode_ink(mode))).push(
            Text::new(mode.label())
                .size(10)
                .font(fonts::JetBrainsMono::SEMI_BOLD)
                .color(color::text_headline()),
        );
    }
    if let Some(label) = world.difficulty.map(Difficulty::label) {
        line = line.push(
            Text::new(format!("· {label}"))
                .size(10)
                .font(fonts::JetBrainsMono::REGULAR)
                .color(color::text_placeholder()),
        );
    }
    line.into()
}

/// A small round dot in `ink`.
fn dot<Message: 'static>(ink: Color) -> Element<'static, Message> {
    container(Space::new().width(6).height(6))
        .style(move |_| container::Style {
            background: Some(Background::Color(ink)),
            border: Border {
                radius: 3.0.into(),
                ..Border::default()
            },
            ..container::Style::default()
        })
        .into()
}

/// The playtime and the size, with their glyphs.
fn stats_line<Message: 'static>(world: &WorldFile) -> Element<'static, Message> {
    let playtime = world.play_time.map_or_else(
        || "—".to_owned(),
        |time| format_playtime(u32::try_from(time.as_secs()).unwrap_or(u32::MAX)),
    );
    row![
        stat(Icon::lucide().clock(), playtime),
        stat(Icon::lucide().hard_drive(), format_bytes(world.size_bytes)),
    ]
    .spacing(12)
    .align_y(alignment::Vertical::Center)
    .into()
}

/// The seed in a read-only field, in the mono line the reference shows.
fn seed_field<Message: 'static>(world: &WorldFile) -> Element<'static, Message> {
    let seed = match world.seed {
        Some(seed) => lang::t_with("worlds.seed", &[("seed", seed.to_string().as_str())]),
        None => lang::t("worlds.seed_none").to_owned(),
    };
    container(
        Text::new(seed)
            .size(11)
            .font(fonts::JetBrainsMono::REGULAR)
            .color(color::text_placeholder()),
    )
    .width(Fill)
    .height(BUTTON_SIZE)
    .padding(Padding::new(0.0).left(12.0).right(12.0))
    .align_y(alignment::Vertical::Center)
    .style(|_| container::Style {
        background: Some(Background::Color(rgb!(255, 255, 255, 3%))),
        border: Border {
            radius: 9.0.into(),
            width: 1.0,
            color: color::surface_line(),
        },
        ..container::Style::default()
    })
    .into()
}

/// The seed and the version a world was saved in, on two mono lines, for the table.
fn seed_and_version<Message: 'static>(world: &WorldFile) -> Element<'static, Message> {
    let seed = match world.seed {
        Some(seed) => lang::t_with("worlds.seed", &[("seed", seed.to_string().as_str())]),
        None => lang::t("worlds.seed_none").to_owned(),
    };
    column![
        Text::new(seed).size(11).font(fonts::JetBrainsMono::REGULAR).color(color::text_muted()),
        Text::new(world.version.clone().unwrap_or_default())
            .size(10)
            .font(fonts::JetBrainsMono::REGULAR)
            .color(color::text_placeholder()),
    ]
    .spacing(2)
    .into()
}

/// The world's ⋯ button: its menu, which the right-click menu lists too.
fn kebab<Message: Clone + 'static>(menu: &Menu, on_select: Rc<dyn Fn(Option<Action>) -> Message>) -> Element<'static, Message> {
    let actions = menu.actions.clone();
    Dropdown::new(Icon::lucide().ellipsis_vertical())
        .items(menu.items.clone())
        .width(BUTTON_SIZE)
        .height(BUTTON_SIZE)
        .on_select(move |index| on_select(actions.get(index).copied().flatten()))
        .into()
}

/// The primary button that loads the world.
fn load_button<Message: Clone + 'static>(play: Message) -> Element<'static, Message> {
    TextButton::primary(lang::t("worlds.load"))
        .padding(Padding::new(0.0).left(14.0).right(14.0).top(7.0).bottom(7.0))
        .on_press(play)
        .into()
}

/// A table row's thumbnail: the save's icon, or a neutral placeholder without one.
pub fn thumbnail<Message: 'static>(icon: Option<&Handle>) -> Element<'static, Message> {
    let art: Element<'static, Message> = match icon {
        Some(handle) => image(handle.clone()).border_radius(10.0).width(THUMB_SIZE).height(THUMB_SIZE).into(),
        None => placeholder_art(10.0),
    };
    container(art).width(THUMB_SIZE).height(THUMB_SIZE).into()
}

/// Wraps a card or row in the context menu that carries its menu rows, keyed by the save's folder.
fn context<Message: Clone + 'static>(
    body: impl Into<Element<'static, Message>>,
    world: &WorldFile,
    menu: Menu,
    on_select: Rc<dyn Fn(Option<Action>) -> Message>,
) -> Element<'static, Message> {
    let Menu { items, actions } = menu;
    ContextMenu::new(body, world.path.display().to_string())
        .items(items)
        .on_select(move |index| on_select(actions.get(index).copied().flatten()))
        .into()
}
