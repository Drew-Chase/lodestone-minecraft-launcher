//! The sidebar — the launcher's primary navigation rail.
//!
//! Matches the reference (`02-library.png`): the Lodestone logo with its
//! glow, a divider, the six primary nav items, the utility items pinned to
//! the bottom, and the player's face with its presence dot. The icon groups
//! reuse [`NavRail`] for the sliding indicator and hover states.
//!
//! Every rail slot navigates and names itself in a tooltip beside it. The logo
//! opens the News home, and its tooltip says so.
//!
//! The Appearance setting can move the same navigation into the floating bar's two
//! segments, from [`floating_segments`], which the floating dock places.

use std::sync::OnceLock;

use crate::app::components::NavRail;
use crate::app::components::game_controls;
use crate::app::components::hint;
use crate::app::lang;
use crate::app::launch::games::GameBar;
use crate::app::theme::{color, fonts, utility};
use aurora_leaf::Icon;
use iced::mouse;
use iced::widget::tooltip::Position;
use iced::widget::{Row, Space, button, column, container, mouse_area, row, stack, svg, text};
use iced::{Background, Border, Color, Element, Fill, Length, Padding, Shadow, Vector, alignment};
use iced_color_extension::rgb;

/// The rail's width — the reference sidebar's 68px.
pub const WIDTH: f32 = 68.0;

/// The padding above the logo. Windows draws its 32px titlebar above the rail, so the logo sits 50px from the top there.
/// macOS draws no titlebar, so its traffic lights sit over the rail's top 32px; the padding takes that strip in too.
const TOP_PADDING: f32 = if cfg!(target_os = "macos") { 50.0 } else { 18.0 };

/// The top nav group: Library, Discover, Worlds, Skins, Servers, Friends. Each entry is the icon's name and the key of its tooltip.
const TOP_ITEMS: [(&str, &str); 6] = [
    ("book", "nav.library"),
    ("compass", "nav.discover"),
    ("globe", "nav.worlds"),
    ("shirt", "nav.skins"),
    ("server", "nav.servers"),
    ("users", "nav.friends"),
];
/// The bottom nav group: Settings. The Downloads trigger sits above it.
const BOTTOM_ITEMS: [(&str, &str); 1] = [("settings", "nav.settings")];

/// The side of a slot in the floating bar, in pixels. An owner keeps its own control to one slot's width.
pub const SLOT_SIZE: f32 = 40.0;

/// The icon markup for a rail slot, resolved once (iconify lookups).
fn rail_icon(name: &str) -> &'static str {
    static CACHE: OnceLock<std::collections::HashMap<&'static str, &'static str>> = OnceLock::new();
    let cache = CACHE.get_or_init(|| {
        [
            ("book", Icon::lucide().book_open()),
            ("compass", Icon::lucide().compass()),
            ("globe", Icon::lucide().globe()),
            ("shirt", Icon::lucide().shirt()),
            ("server", Icon::lucide().server()),
            ("users", Icon::lucide().users()),
            ("settings", Icon::lucide().settings()),
        ]
        .into_iter()
        .collect()
    });
    cache.get(name).copied().unwrap_or("")
}

/// The rail's slots: each icon, with the tooltip its key names.
fn rail_items(items: &[(&'static str, &'static str)]) -> Vec<crate::app::components::NavItem> {
    items
        .iter()
        .map(|&(name, label)| crate::app::components::NavItem::labelled(rail_icon(name), lang::t(label)))
        .collect()
}

/// The full sidebar: logo, nav groups, and the account's face. `account` is the face and its menu, already named in a
/// hint, which the caller builds. `active` is the highlighted index in the top group; the closures route the two groups'
/// selections, and `on_logo` is what a press on the logo sends. `above_bottom` holds the game block and the Downloads
/// trigger, placed above the bottom group.
pub fn sidebar<Message: Clone + 'static>(
    account: Element<'static, Message>,
    active: usize,
    on_top_nav: impl Fn(usize) -> Message + 'static,
    on_bottom_nav: impl Fn(usize) -> Message + 'static,
    on_logo: Message,
    above_bottom: Element<'static, Message>,
) -> Element<'static, Message> {
    let top_rail = NavRail::new(rail_items(&TOP_ITEMS)).active(active).indicator(false).on_select(on_top_nav);
    let bottom_rail = NavRail::new(rail_items(&BOTTOM_ITEMS)).indicator(false).on_select(on_bottom_nav);

    let rail = column![logo(on_logo), divider(), top_rail, Space::new().height(Fill), above_bottom]
        .spacing(6)
        .push(bottom_rail)
        .push(centered(account));

    container(rail.padding(Padding::new(0.0).top(TOP_PADDING).bottom(18)))
        .width(WIDTH)
        .height(Fill)
        .style(|_| container::Style {
            background: Some(Background::Color(color::surface_bg1())),
            ..container::Style::default()
        })
        .into()
}

/// The floating bar's two segments, for the floating dock to place. The first runs from News to Friends. The second holds
/// the running games, the Downloads trigger, Settings and the account's face. `active` is the highlighted top item, and
/// `news` highlights the logo instead while the News home shows. The closures route the top group and the segments'
/// buttons, as [`sidebar`] does.
pub fn floating_segments<Message: Clone + 'static>(
    active: Option<usize>,
    news: bool,
    on_top_nav: impl Fn(usize) -> Message,
    on_logo: Message,
    on_settings: Message,
    running: Element<'static, Message>,
    account: Element<'static, Message>,
) -> (Element<'static, Message>, Element<'static, Message>) {
    let mut main: Row<'static, Message> = row![bar_logo(on_logo, news)].spacing(4).align_y(alignment::Vertical::Center);
    for (index, &(icon, label)) in TOP_ITEMS.iter().enumerate() {
        main = main.push(bar_slot(rail_icon(icon), lang::t(label), active == Some(index), on_top_nav(index)));
    }
    let side = row![
        running,
        bar_slot(rail_icon("settings"), lang::t(BOTTOM_ITEMS[0].1), false, on_settings),
        account
    ]
    .spacing(4)
    .align_y(alignment::Vertical::Center);
    (segment(main), segment(side))
}

/// The game block above the Downloads item: the indicator, whose panel lists the running games with
/// their console and kill buttons, and a button on each row that starts another run of its instance.
/// Starting a run of an instance that is not running is left to the instance and library pages.
pub fn game_block<Message: Clone + 'static>(bar: &GameBar, open: bool, actions: &game_controls::Actions<Message>) -> Element<'static, Message> {
    centered(game_indicator(bar, open, actions, Position::Right))
}

/// The running-games indicator and its panel, named in a hint at `position`.
pub fn game_indicator<Message: Clone + 'static>(
    bar: &GameBar,
    open: bool,
    actions: &game_controls::Actions<Message>,
    position: Position,
) -> Element<'static, Message> {
    hint::at(game_controls::runs_popover(bar, open, actions), lang::t("nav.running_games"), position)
}

/// Centres `content` across the rail.
fn centered<Message: 'static>(content: impl Into<Element<'static, Message>>) -> Element<'static, Message> {
    container(content).width(Fill).center_x(Fill).into()
}

/// The Lodestone mark with its glow halo. A press sends `on_press`, which opens the News home, and the tooltip names it.
fn logo<Message: Clone + 'static>(on_press: Message) -> Element<'static, Message> {
    let mark = container(
        stack![
            container(svg(crate::app::components::backdrop::logo_glow_handle().clone()).width(72).height(72))
                .width(Fill)
                .height(Fill)
                .align_x(alignment::Horizontal::Center)
                .align_y(alignment::Vertical::Center),
            svg(crate::app::pages::login::logo_handle().clone()).width(40).height(40),
        ]
        .width(40)
        .height(40),
    )
    .width(Fill)
    .center_x(Length::Fill);
    hint::at(
        mouse_area(mark).on_press(on_press).interaction(mouse::Interaction::Pointer),
        lang::t("nav.news"),
        Position::Right,
    )
}

/// The mark at the start of the floating bar: the logo with its word beside it, opening the News home.
fn bar_logo<Message: Clone + 'static>(on_press: Message, selected: bool) -> Element<'static, Message> {
    let mark = row![
        svg(crate::app::pages::login::logo_handle().clone()).width(22).height(22),
        bar_word(lang::t("nav.news"), selected),
    ]
    .spacing(8)
    .align_y(alignment::Vertical::Center);
    bar_button(mark, selected, on_press)
}

/// One labelled slot of the floating bar: its icon with the word beside it. The selected slot takes the accent tint.
fn bar_slot<Message: Clone + 'static>(icon: &'static str, label: &'static str, selected: bool, on_press: Message) -> Element<'static, Message> {
    let ink = if selected { color::primary() } else { color::text_muted() };
    let content = row![utility::icon(icon, 18.0, Some((ink, ink))), bar_word(label, selected)]
        .spacing(8)
        .align_y(alignment::Vertical::Center);
    bar_button(content, selected, on_press)
}

/// The word of a floating bar slot.
fn bar_word<'a, Message: 'a>(label: &'static str, selected: bool) -> Element<'a, Message> {
    let ink = if selected { color::primary() } else { color::text_muted() };
    text(label).size(13).font(fonts::Inter::MEDIUM).color(ink).into()
}

/// The button a floating bar slot's content sits in: 40px tall, rounded, tinted when selected, faint white under the pointer.
fn bar_button<'a, Message: Clone + 'a>(content: impl Into<Element<'a, Message>>, selected: bool, on_press: Message) -> Element<'a, Message> {
    // The content sits centred on the slot's height, so the word and the icon share a midline.
    button(container(content.into()).height(Fill).align_y(alignment::Vertical::Center))
        .height(SLOT_SIZE)
        .padding(Padding::new(0.0).left(12.0).right(14.0))
        .on_press(on_press)
        .style(move |_, status| button::Style {
            background: Some(Background::Color(match (selected, status) {
                (true, _) => color::primary_tint(),
                (false, button::Status::Hovered | button::Status::Pressed) => rgb!(255, 255, 255, 8%),
                _ => Color::TRANSPARENT,
            })),
            border: Border {
                radius: 12.0.into(),
                ..Border::default()
            },
            text_color: color::text_headline(),
            ..button::Style::default()
        })
        .into()
}

/// One floating segment: its content on a translucent panel with a lighter hairline and rounded corners, and a soft
/// shadow when the glass effect is on.
fn segment<'a, Message: 'a>(content: impl Into<Element<'a, Message>>) -> Element<'a, Message> {
    container(content)
        .padding(Padding::new(6.0))
        .style(|_| container::Style {
            background: Some(Background::Color(Color {
                a: 0.96,
                ..color::surface_bg1()
            })),
            border: Border {
                radius: 18.0.into(),
                width: 1.0,
                color: color::surface_line_strong(),
            },
            shadow: Shadow {
                color: rgb!(0, 0, 0, 45%),
                offset: Vector::new(0.0, 8.0),
                blur_radius: crate::app::theme::blur(24.0),
            },
            ..container::Style::default()
        })
        .into()
}

/// The 32px hairline under the logo.
fn divider<Message: Clone + 'static>() -> Element<'static, Message> {
    container(Space::new().height(1).width(32))
        .width(Fill)
        .style(|_| container::Style {
            background: Some(Background::Color(color::surface_line())),
            ..container::Style::default()
        })
        .into()
}
