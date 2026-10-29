//! Modal — the full-window dialog from the quick-action references
//! (`res/reference-images/11–14-modal-*.png`).
//!
//! A modal dims the whole window behind a scrim and centres a card with three
//! slots: a title, a body and an optional footer. Every slot takes plain text,
//! which is styled for its slot, or any element (a row, column, container…).
//! The card is the style guide's "Popover / Modal" elevation: `--bg-3`, a
//! `--line-strong` border, the deep shadow and the `--r-xl` radius.
//!
//! ```text
//! Modal::new("New Instance", body_column)
//!     .subtitle("Spin up a fresh Minecraft profile.")
//!     .icon(Icon::lucide().plus(), color::Primary::DEFAULT, color::Primary::TINT)
//!     .footer(row![Space::new().width(Fill), cancel, create].spacing(12))
//!     .on_close(Message::CloseModal)
//! ```
//!
//! The modal is stateless: the owner keeps whether it is open. A page exposes
//! `fn modal(&self) -> Option<Element<'_, Message>>` and the root stacks it
//! above the titlebar and sidebar in `view.rs`, so the scrim covers the whole
//! window. While a modal is up it swallows every mouse, keyboard and touch
//! event, so nothing underneath reacts (see `layer.rs`). For yes/no questions
//! use [`super::alert`], which builds on this.

mod layer;

use crate::app::components::icon_button::{self, IconButton};
use crate::app::components::typography;
use crate::app::theme::utility;
use aurora_leaf::Icon;
use iced::widget::{Column, Container, Row, Text, column, container, row, scrollable};
use iced::{Background, Border, Color, Element, Fill, Length, Padding, alignment};
use std::borrow::Cow;

use layer::Layer;

/// The card's width when none is given — the alert size.
pub const DEFAULT_WIDTH: f32 = 560.0;

/// Header padding: the title block sits 28px in from the card edge.
const HEADER_PADDING: Padding = Padding {
    top: 24.0,
    bottom: 22.0,
    left: 28.0,
    right: 24.0,
};
const BODY_PADDING: Padding = Padding {
    top: 24.0,
    bottom: 24.0,
    left: 28.0,
    right: 28.0,
};
const FOOTER_PADDING: Padding = Padding {
    top: 18.0,
    bottom: 18.0,
    left: 28.0,
    right: 28.0,
};

/// One modal slot: text, styled for the slot it lands in, or an element
/// rendered as given.
pub enum Slot<'a, Message> {
    Text(Cow<'a, str>),
    Element(Element<'a, Message>),
}

impl<'a, Message> From<&'a str> for Slot<'a, Message> {
    fn from(text: &'a str) -> Self {
        Self::Text(Cow::Borrowed(text))
    }
}

impl<Message> From<String> for Slot<'_, Message> {
    fn from(text: String) -> Self {
        Self::Text(Cow::Owned(text))
    }
}

impl<'a, Message> From<Cow<'a, str>> for Slot<'a, Message> {
    fn from(text: Cow<'a, str>) -> Self {
        Self::Text(text)
    }
}

impl<'a, Message> From<Element<'a, Message>> for Slot<'a, Message> {
    fn from(element: Element<'a, Message>) -> Self {
        Self::Element(element)
    }
}

impl<'a, Message: 'a> From<Row<'a, Message>> for Slot<'a, Message> {
    fn from(row: Row<'a, Message>) -> Self {
        Self::Element(row.into())
    }
}

impl<'a, Message: 'a> From<Column<'a, Message>> for Slot<'a, Message> {
    fn from(column: Column<'a, Message>) -> Self {
        Self::Element(column.into())
    }
}

impl<'a, Message: 'a> From<Container<'a, Message>> for Slot<'a, Message> {
    fn from(container: Container<'a, Message>) -> Self {
        Self::Element(container.into())
    }
}

/// A `Text` keeps the styling it was built with.
impl<'a, Message: 'a> From<Text<'a>> for Slot<'a, Message> {
    fn from(text: Text<'a>) -> Self {
        Self::Element(text.into())
    }
}

impl<'a, Message: 'a> Slot<'a, Message> {
    /// Renders the slot, styling text with `style`.
    fn into_element(self, style: fn(Cow<'a, str>) -> Text<'a>) -> Element<'a, Message> {
        match self {
            Slot::Text(text) => style(text).into(),
            Slot::Element(element) => element,
        }
    }
}

/// The optional glyph tile beside the title.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct HeaderIcon {
    pub svg: &'static str,
    pub ink: Color,
    pub tint: Color,
}

/// A full-window modal dialog.
pub struct Modal<'a, Message> {
    title: Slot<'a, Message>,
    subtitle: Option<Cow<'a, str>>,
    icon: Option<HeaderIcon>,
    body: Slot<'a, Message>,
    footer: Option<Slot<'a, Message>>,
    on_close: Option<Message>,
    close_on_scrim: bool,
    width: f32,
}

impl<'a, Message: Clone + 'a> Modal<'a, Message> {
    /// A modal with a title and a body. Text titles render as `h3`, text
    /// bodies as `body`.
    pub fn new(title: impl Into<Slot<'a, Message>>, body: impl Into<Slot<'a, Message>>) -> Self {
        Self {
            title: title.into(),
            subtitle: None,
            icon: None,
            body: body.into(),
            footer: None,
            on_close: None,
            close_on_scrim: true,
            width: DEFAULT_WIDTH,
        }
    }

    /// A muted line under the title.
    pub fn subtitle(mut self, subtitle: impl Into<Cow<'a, str>>) -> Self {
        self.subtitle = Some(subtitle.into());
        self
    }

    /// A 52px glyph tile before the title: `ink` draws the glyph and the
    /// border, `tint` fills the tile.
    pub fn icon(mut self, svg: &'static str, ink: Color, tint: Color) -> Self {
        self.icon = Some(HeaderIcon { svg, ink, tint });
        self
    }

    /// The bar under the body, usually the action buttons. Text renders as
    /// a `small` note.
    pub fn footer(mut self, footer: impl Into<Slot<'a, Message>>) -> Self {
        self.footer = Some(footer.into());
        self
    }

    /// The message sent by the ✕ button, Esc and (unless disabled with
    /// [`Self::close_on_scrim`]) a click on the scrim. Without it the modal
    /// has no ✕ and only its own controls can close it.
    pub fn on_close(mut self, message: Message) -> Self {
        self.on_close = Some(message);
        self
    }

    /// Whether a click on the scrim sends the `on_close` message.
    pub fn close_on_scrim(mut self, close: bool) -> Self {
        self.close_on_scrim = close;
        self
    }

    /// The card's width. A narrower window shrinks it to fit.
    pub fn width(mut self, width: f32) -> Self {
        self.width = width;
        self
    }

    /// The icon tile, title block and ✕.
    fn header(title: Slot<'a, Message>, subtitle: Option<Cow<'a, str>>, icon: Option<HeaderIcon>, on_close: Option<Message>) -> Element<'a, Message> {
        let mut heading = column![title.into_element(typography::h3)].spacing(4).width(Fill);
        if let Some(subtitle) = subtitle {
            heading = heading.push(typography::small(subtitle));
        }

        let mut header = row![].spacing(16).align_y(alignment::Vertical::Center);
        if let Some(icon) = icon {
            header = header.push(icon_tile(icon));
        }
        header = header.push(heading);
        if let Some(message) = on_close {
            header = header.push(
                IconButton::new(Icon::lucide().x())
                    .size(36)
                    .icon_size(14.0)
                    .radius(10.0)
                    .style(icon_button::SECONDARY_BUTTON_STYLE)
                    .on_press(message),
            );
        }
        container(header).width(Fill).padding(HEADER_PADDING).into()
    }
}

/// The tinted glyph tile from the reference headers.
fn icon_tile<'a, Message: 'a>(icon: HeaderIcon) -> Element<'a, Message> {
    let HeaderIcon { svg, ink, tint } = icon;
    container(utility::icon(svg, 22.0, Some((ink, ink))))
        .width(52)
        .height(52)
        .align_x(alignment::Horizontal::Center)
        .align_y(alignment::Vertical::Center)
        .style(move |_| container::Style {
            background: Some(Background::Color(tint)),
            border: Border {
                radius: 14.0.into(),
                width: 1.0,
                color: Color { a: 0.35, ..ink },
            },
            ..container::Style::default()
        })
        .into()
}

impl<'a, Message: Clone + 'a> From<Modal<'a, Message>> for Element<'a, Message> {
    fn from(modal: Modal<'a, Message>) -> Self {
        let Modal {
            title,
            subtitle,
            icon,
            body,
            footer,
            on_close,
            close_on_scrim,
            width,
        } = modal;

        let header = Modal::header(title, subtitle, icon, on_close.clone());
        // The layer hands the body whatever height the header and footer
        // leave, so a long body scrolls instead of pushing the footer out.
        let body = scrollable(container(body.into_element(typography::body)).width(Fill).padding(BODY_PADDING))
            .width(Fill)
            .height(Length::Shrink)
            .into();
        let footer = footer.map(|footer| {
            container(footer.into_element(typography::small))
                .width(Fill)
                .padding(FOOTER_PADDING)
                .into()
        });

        Element::new(Layer::new(header, body, footer, width, on_close, close_on_scrim))
    }
}

#[cfg(test)]
#[path = "../../../../tests/app/components/modal/mod.rs"]
mod tests;
