//! The sidebar — the launcher's primary navigation rail.
//!
//! Matches the reference (`02-library.png`): the Lodestone logo with its
//! glow, a divider, the five primary nav items, the utility items pinned to
//! the bottom, and the player avatar with its presence dot. The icon groups
//! reuse [`NavRail`] for the sliding indicator and hover states.
//!
//! Only Home navigates today; the remaining items keep their hover
//! affordances but produce no navigation.

use std::sync::OnceLock;

use crate::app::components::NavRail;
use crate::app::components::icon_button::{self, IconButton};
use crate::app::theme::color;
use crate::app::theme::fonts;
use aurora_leaf::Icon;
use iced::widget::image::Handle;
use iced::widget::{Space, Text, column, container, image, row, stack, svg};
use iced::{Background, Border, Element, Fill, Length, Padding, alignment};
use iced_color_extension::rgb;

use crate::app::face::AVATAR_RADIUS;

/// The rail's width — the reference sidebar's 68px.
pub const WIDTH: f32 = 68.0;

/// The top nav group: Library, Discover, Worlds, Servers, Friends.
const TOP_ICONS: [&str; 5] = ["home", "compass", "globe", "server", "users"];
/// The bottom nav group: Downloads, Settings.
const BOTTOM_ICONS: [&str; 2] = ["download", "settings"];

/// The icon markup for a rail slot, resolved once (iconify lookups).
fn rail_icon(name: &str) -> &'static str {
    static CACHE: OnceLock<std::collections::HashMap<&'static str, &'static str>> = OnceLock::new();
    let cache = CACHE.get_or_init(|| {
        [
            ("home", Icon::lucide().house()),
            ("compass", Icon::lucide().compass()),
            ("globe", Icon::lucide().globe()),
            ("server", Icon::lucide().server()),
            ("users", Icon::lucide().users()),
            ("download", Icon::lucide().download()),
            ("settings", Icon::lucide().settings()),
        ]
        .into_iter()
        .collect()
    });
    cache.get(name).copied().unwrap_or("")
}

fn rail_items(names: &[&'static str]) -> Vec<crate::app::components::NavItem> {
    names.iter().map(|name| crate::app::components::NavItem::new(rail_icon(name))).collect()
}

/// The full sidebar: logo, nav groups, and the player avatar. `active` is
/// the highlighted index in the top group; the closures route the two
/// groups' selections. `game` is the running game's controls, placed above the bottom group.
pub fn sidebar<Message: Clone + 'static>(
    face: Option<Handle>,
    username: Option<String>,
    online: bool,
    active: usize,
    on_top_nav: impl Fn(usize) -> Message + 'static,
    on_bottom_nav: impl Fn(usize) -> Message + 'static,
    game: Option<Element<'static, Message>>,
) -> Element<'static, Message> {
    let top_rail = NavRail::new(rail_items(&TOP_ICONS)).active(active).indicator(false).on_select(on_top_nav);
    let bottom_rail = NavRail::new(rail_items(&BOTTOM_ICONS)).indicator(false).on_select(on_bottom_nav);

    let mut rail = column![logo(), divider(), top_rail, Space::new().height(Fill)].spacing(6);
    if let Some(game) = game {
        rail = rail.push(game);
    }
    let rail = rail.push(bottom_rail).push(avatar(face, username, online));

    container(rail.padding(Padding::new(0.0).top(18).bottom(18)))
        .width(WIDTH)
        .height(Fill)
        .style(|_| container::Style {
            background: Some(Background::Color(color::Surface::BG1)),
            ..container::Style::default()
        })
        .into()
}

/// The two small buttons that stop the running game and open its console, side by side above the
/// Downloads item.
pub fn game_controls<Message: Clone + 'static>(kill: Message, console: Message) -> Element<'static, Message> {
    container(
        row![
            IconButton::new(Icon::lucide().square())
                .width(28)
                .height(28)
                .icon_size(11.0)
                .style(icon_button::DANGER_BUTTON_STYLE)
                .on_press(kill),
            IconButton::new(Icon::lucide().terminal())
                .width(28)
                .height(28)
                .icon_size(12.0)
                .style(icon_button::SECONDARY_BUTTON_STYLE)
                .on_press(console),
        ]
        .spacing(6),
    )
    .width(Fill)
    .center_x(Fill)
    .into()
}

/// The Lodestone mark with its glow halo.
fn logo<Message: Clone + 'static>() -> Element<'static, Message> {
    container(
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
    .center_x(Length::Fill)
    .into()
}

/// The 32px hairline under the logo.
fn divider<Message: Clone + 'static>() -> Element<'static, Message> {
    container(Space::new().height(1).width(32))
        .width(Fill)
        .style(|_| container::Style {
            background: Some(Background::Color(color::Surface::LINE)),
            ..container::Style::default()
        })
        .into()
}

/// The player avatar: the 8×8 face upscaled to 40×40 with the presence dot
/// pinned to its bottom-right corner. Falls back to the username's initial
/// on the green gradient when no face is loaded.
fn avatar<Message: Clone + 'static>(face: Option<Handle>, username: Option<String>, online: bool) -> Element<'static, Message> {
    let face_layer: Element<'static, Message> = match face {
        Some(handle) => image(handle).width(40).height(40).into(),
        None => {
            let initial = username
                .as_deref()
                .and_then(|name| name.chars().next())
                .map(|initial| initial.to_uppercase().to_string())
                .unwrap_or_default();
            container(Text::new(initial).size(16).font(fonts::Inter::EXTRA_BOLD).color(color::Primary::ON))
                .width(Fill)
                .height(Fill)
                .center(Length::Fill)
                .into()
        }
    };

    let dot_ink = if online { color::Primary::DEFAULT } else { color::Text::DISABLED };

    container(
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
                    color: color::Surface::BG1,
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
    .center_x(Fill)
    .into()
}
