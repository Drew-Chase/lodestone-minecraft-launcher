//! The Settings sidebar: the six tabs, with the active one lit by a green bar, and a link to the component gallery at its
//! foot.

use iced::widget::{Space, button, column, container, row, text};
use iced::{Background, Border, Element, Fill, Padding, alignment};

use super::{Message, Tab};
use crate::app::components::TextButton;
use crate::app::lang;
use crate::app::pages::Page;
use crate::app::theme::{color, fonts, utility};

/// The sidebar's width.
const WIDTH: f32 = 260.0;

/// The sidebar: an eyebrow, the six tabs, and the component gallery link.
pub fn view<'a>(active: Tab) -> Element<'a, Message> {
    let tabs = Tab::ALL
        .into_iter()
        .fold(column![].spacing(4), |list, tab| list.push(item(tab, tab == active)));
    column![
        crate::eyebrow!(lang::t("settings.nav.eyebrow")),
        tabs,
        Space::new().height(Fill),
        TextButton::ghost(lang::t("settings.component_gallery")).on_press(Message::Navigate(Page::Showcase)),
    ]
    .spacing(18)
    .padding(Padding::new(0.0).top(30.0).bottom(24.0).left(16.0).right(16.0))
    .width(WIDTH)
    .height(Fill)
    .into()
}

/// One tab: its icon and title, lit when it is the active tab.
fn item<'a>(tab: Tab, active: bool) -> Element<'a, Message> {
    let ink = if active { color::primary() } else { color::text_muted() };
    let bar = container(Space::new().width(3).height(20)).style(move |_| container::Style {
        background: active.then_some(Background::Color(color::primary())),
        ..container::Style::default()
    });
    let label = text(lang::t(tab.title_key())).size(14).font(fonts::Inter::MEDIUM).color(ink);
    let content = row![bar, utility::icon(tab.icon(), 16.0, Some((ink, ink))), label]
        .spacing(12)
        .align_y(alignment::Vertical::Center);
    button(content)
        .on_press(Message::SelectTab(tab))
        .width(Fill)
        .padding(Padding::new(0.0).top(10.0).bottom(10.0).right(12.0))
        .style(move |_, status| {
            let hovered = matches!(status, button::Status::Hovered | button::Status::Pressed);
            let background = if active {
                Some(Background::Color(color::primary_tint()))
            } else if hovered {
                Some(Background::Color(color::surface_bg4()))
            } else {
                None
            };
            button::Style {
                background,
                text_color: ink,
                border: Border {
                    radius: 10.0.into(),
                    ..Border::default()
                },
                ..button::Style::default()
            }
        })
        .into()
}
