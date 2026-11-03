//! The sidebar — the launcher's primary navigation rail.
//!
//! Matches the reference (`02-library.png`): the Lodestone logo with its
//! glow, a divider, the six primary nav items, the utility items pinned to
//! the bottom, and the player avatar with its presence dot. The icon groups
//! reuse [`NavRail`] for the sliding indicator and hover states.
//!
//! Every rail slot navigates and names itself in a tooltip beside it. The logo
//! opens the News home, and its tooltip says so.

use std::sync::OnceLock;

use crate::app::components::NavRail;
use crate::app::components::game_controls;
use crate::app::components::hint;
use crate::app::lang;
use crate::app::launch::games::GameBar;
use crate::app::theme::color;
use crate::app::theme::fonts;
use aurora_leaf::Icon;
use iced::mouse;
use iced::widget::image::Handle;
use iced::widget::tooltip::Position;
use iced::widget::{Space, Text, column, container, image, mouse_area, stack, svg};
use iced::{Background, Border, Element, Fill, Length, Padding, alignment};
use iced_color_extension::rgb;

use crate::app::face::AVATAR_RADIUS;

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

/// The full sidebar: logo, nav groups, and the player avatar. `active` is
/// the highlighted index in the top group; the closures route the two
/// groups' selections, and `on_logo` is what a press on the logo sends.
/// `above_bottom` holds the game block and the Downloads trigger, placed
/// above the bottom group. Each part is its own argument, so there are eight.
#[allow(clippy::too_many_arguments)]
pub fn sidebar<Message: Clone + 'static>(
    face: Option<Handle>,
    username: Option<String>,
    online: bool,
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
        .push(avatar(face, username, online));

    container(rail.padding(Padding::new(0.0).top(TOP_PADDING).bottom(18)))
        .width(WIDTH)
        .height(Fill)
        .style(|_| container::Style {
            background: Some(Background::Color(color::surface_bg1())),
            ..container::Style::default()
        })
        .into()
}

/// The game block above the Downloads item: the indicator, whose panel lists the running games with
/// their console and kill buttons, and a button on each row that starts another run of its instance.
/// Starting a run of an instance that is not running is left to the instance and library pages.
pub fn game_block<Message: Clone + 'static>(bar: &GameBar, open: bool, actions: &game_controls::Actions<Message>) -> Element<'static, Message> {
    centered(hint::at(
        game_controls::runs_popover(bar, open, actions),
        lang::t("nav.running_games"),
        Position::Right,
    ))
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

/// The player avatar: the 8×8 face upscaled to 40×40 with the presence dot
/// pinned to its bottom-right corner. Falls back to the username's initial
/// on the green gradient when no face is loaded. Its tooltip sits beside it.
fn avatar<Message: Clone + 'static>(face: Option<Handle>, username: Option<String>, online: bool) -> Element<'static, Message> {
    let face_layer: Element<'static, Message> = match face {
        Some(handle) => image(handle).width(40).height(40).into(),
        None => {
            let initial = username
                .as_deref()
                .and_then(|name| name.chars().next())
                .map(|initial| initial.to_uppercase().to_string())
                .unwrap_or_default();
            container(Text::new(initial).size(16).font(fonts::Inter::EXTRA_BOLD).color(color::primary_on()))
                .width(Fill)
                .height(Fill)
                .center(Length::Fill)
                .into()
        }
    };

    let dot_ink = if online { color::primary() } else { color::text_disabled() };

    let face = container(
        stack![
            face_layer,
            // The ring rides on top of the face; the corners themselves are
            // faded in the texture.
            container(Space::new().width(Fill).height(Fill)).style(|_| container::Style {
                border: Border {
                    radius: (AVATAR_RADIUS + 2.0).into(),
                    width: 1.5,
                    color: rgb!(255, 255, 255, 25%),
                },
                ..container::Style::default()
            }),
            container(container(Space::new().width(10).height(10)).style(move |_| container::Style {
                background: Some(Background::Color(dot_ink)),
                border: Border {
                    radius: 5.0.into(),
                    width: 2.0,
                    color: color::surface_bg1(),
                },
                ..container::Style::default()
            }))
            .width(Fill)
            .height(Fill)
            .align_x(alignment::Horizontal::Right)
            .align_y(alignment::Vertical::Bottom)
            .padding(Padding::new(0.0).right(1).bottom(1)),
        ]
        .width(40)
        .height(40),
    )
    .width(Fill)
    .center_x(Fill);
    hint::at(face, lang::t("nav.account"), Position::Right)
}
