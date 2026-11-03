//! The Settings sidebar: the six sections, with the one at the top of the page lit, and a link to the component gallery at its
//! foot. A press scrolls the page to its section, and the page's scroll lights the section it reaches.

use iced::widget::scrollable::Viewport;
use iced::widget::{Id, Space, button, column, container, row, text};
use iced::{Background, Border, Element, Fill, Padding, Task, alignment};

use super::{Message, Tab};
use crate::app::components::TextButton;
use crate::app::components::scroll_spy;
use crate::app::lang;
use crate::app::pages::Page;
use crate::app::theme::{color, fonts, utility};

/// The sidebar's width.
const WIDTH: f32 = 260.0;

/// The id of the page's scrollable.
pub fn content_id() -> Id {
    Id::new("settings-content")
}

/// The id of the column of sections inside the scrollable. Its top is the zero the sections are measured from.
pub fn column_id() -> Id {
    Id::new("settings-column")
}

/// Reads a scroll into the page's message: the offset, and whether the page is at its bottom with more above.
pub fn on_scroll(viewport: Viewport) -> Message {
    let (offset, at_end) = scroll_spy::position(viewport);
    Message::Scrolled { offset, at_end }
}

/// Measures where each section's top sits on the page.
pub fn measure() -> Task<Message> {
    scroll_spy::measure(column_id(), anchors()).map(Message::Measured)
}

/// Scrolls the page so `tab` sits at its top.
pub fn scroll_to(tab: Tab) -> Task<Message> {
    scroll_spy::scroll_to(tab, content_id(), column_id(), anchors())
}

/// Every section with its anchor, in page order.
fn anchors() -> Vec<(Tab, Id)> {
    Tab::ALL.into_iter().map(|tab| (tab, tab.anchor_id())).collect()
}

/// The sidebar: an eyebrow, the six sections, and the component gallery link.
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

/// One section: its icon and title, lit when it is the section at the top of the page.
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
        .on_press(Message::Section(tab))
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
