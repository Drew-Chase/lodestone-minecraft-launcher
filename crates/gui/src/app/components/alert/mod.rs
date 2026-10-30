//! Alerts — modals with a preset title, a message body and a preset footer,
//! answering a yes/no question with a `bool`.
//!
//! An [`Alert`] is plain data; the root-owned [`Alerts`] host queues and
//! shows them. [`Alerts::show`] returns a `Task<bool>` that resolves when the
//! alert closes: `true` for Yes, Okay and Close; `false` for No, Cancel, Esc
//! and the ✕. The scrim does not close an alert, so a stray click never
//! answers a destructive question.
//!
//! ```text
//! Alert::danger("Delete “Aether Legacy”? This cannot be undone.").title("Delete instance")
//! Alert::warn("Java 17 is older than this version expects.").ok_cancel()
//! Alert::error("The download failed.")                       // Close
//! ```

#![allow(dead_code, reason = "no page raises an alert yet; remove this once one does")]

mod host;

use crate::app::components::button::{TextButton, Variant};
use crate::app::components::modal::Modal;
use crate::app::lang;
use crate::app::theme::color;
use aurora_leaf::Icon;
use iced::widget::{Space, row};
use iced::{Color, Element, Fill, alignment};
use std::borrow::Cow;

pub use host::Alerts;

/// The answer an alert sends back to the root.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Message {
    Respond(bool),
}

/// What an alert is about; sets its default title, glyph and colours.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Kind {
    Info,
    Warning,
    Error,
    /// A destructive confirmation: the affirmative button is red.
    Danger,
}

impl Kind {
    pub fn title(self) -> &'static str {
        match self {
            Kind::Info => lang::t("alert.title.info"),
            Kind::Warning => lang::t("alert.title.warning"),
            Kind::Error => lang::t("alert.title.error"),
            Kind::Danger => lang::t("alert.title.danger"),
        }
    }

    pub fn icon(self) -> &'static str {
        match self {
            Kind::Info => Icon::lucide().info(),
            Kind::Warning | Kind::Danger => Icon::lucide().triangle_alert(),
            Kind::Error => Icon::lucide().circle_x(),
        }
    }

    /// The glyph ink and the tile fill.
    pub fn palette(self) -> (Color, Color) {
        match self {
            Kind::Info => (color::Accents::CYAN, color::Accents::CYAN_TINT),
            Kind::Warning => (color::Accents::AMBER, color::Accents::AMBER_TINT),
            Kind::Error => (color::Accents::PINK, color::Accents::PINK_TINT),
            Kind::Danger => {
                let red = color::buttons::Danger::BACKGROUND_HOVER;
                (red, Color { a: 0.14, ..red })
            }
        }
    }

    /// The buttons an alert of this kind starts with.
    pub fn default_buttons(self) -> Buttons {
        match self {
            Kind::Info | Kind::Error => Buttons::Close,
            Kind::Warning => Buttons::OkCancel,
            Kind::Danger => Buttons::YesNo,
        }
    }
}

/// The footer's button set.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Buttons {
    YesNo,
    OkCancel,
    Close,
}

impl Buttons {
    /// The affirmative label, which answers `true`.
    pub fn affirmative(self) -> &'static str {
        match self {
            Buttons::YesNo => lang::t("common.yes"),
            Buttons::OkCancel => lang::t("common.okay"),
            Buttons::Close => lang::t("common.close"),
        }
    }

    /// The negative label, which answers `false`. `Close` has none.
    pub fn negative(self) -> Option<&'static str> {
        match self {
            Buttons::YesNo => Some(lang::t("common.no")),
            Buttons::OkCancel => Some(lang::t("common.cancel")),
            Buttons::Close => None,
        }
    }
}

/// A question or notice shown in a modal.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Alert {
    kind: Kind,
    title: Cow<'static, str>,
    message: Cow<'static, str>,
    buttons: Buttons,
}

impl Alert {
    pub fn new(kind: Kind, message: impl Into<Cow<'static, str>>) -> Self {
        Self {
            kind,
            title: Cow::Borrowed(kind.title()),
            message: message.into(),
            buttons: kind.default_buttons(),
        }
    }

    /// An info notice with a Close button.
    pub fn info(message: impl Into<Cow<'static, str>>) -> Self {
        Self::new(Kind::Info, message)
    }

    /// A warning with Okay / Cancel.
    pub fn warn(message: impl Into<Cow<'static, str>>) -> Self {
        Self::new(Kind::Warning, message)
    }

    /// An error notice with a Close button.
    pub fn error(message: impl Into<Cow<'static, str>>) -> Self {
        Self::new(Kind::Error, message)
    }

    /// A destructive confirmation with Yes / No and a red Yes.
    pub fn danger(message: impl Into<Cow<'static, str>>) -> Self {
        Self::new(Kind::Danger, message)
    }

    /// Replaces the kind's default title.
    pub fn title(mut self, title: impl Into<Cow<'static, str>>) -> Self {
        self.title = title.into();
        self
    }

    pub fn yes_no(self) -> Self {
        self.buttons(Buttons::YesNo)
    }

    pub fn ok_cancel(self) -> Self {
        self.buttons(Buttons::OkCancel)
    }

    pub fn close(self) -> Self {
        self.buttons(Buttons::Close)
    }

    pub fn buttons(mut self, buttons: Buttons) -> Self {
        self.buttons = buttons;
        self
    }

    pub fn kind(&self) -> Kind {
        self.kind
    }

    pub fn button_set(&self) -> Buttons {
        self.buttons
    }

    pub fn heading(&self) -> &str {
        &self.title
    }

    pub fn message(&self) -> &str {
        &self.message
    }

    /// The affirmative button's look: red for a destructive question.
    fn affirmative_variant(&self) -> Variant {
        if self.kind == Kind::Danger { Variant::Danger } else { Variant::Primary }
    }

    /// The alert as a modal: kind glyph and title, the message, and the
    /// button set right-aligned in the footer.
    pub fn view(&self) -> Element<'_, Message> {
        let (ink, tint) = self.kind.palette();

        let mut footer = row![Space::new().width(Fill)].spacing(12).align_y(alignment::Vertical::Center);
        if let Some(negative) = self.buttons.negative() {
            footer = footer.push(TextButton::ghost(negative).on_press(Message::Respond(false)));
        }
        let affirmative = match self.affirmative_variant() {
            Variant::Primary => TextButton::primary(self.buttons.affirmative()),
            variant => TextButton::ghost(self.buttons.affirmative()).variant(variant),
        };
        footer = footer.push(affirmative.on_press(Message::Respond(true)));

        Modal::new(self.heading(), self.message())
            .icon(self.kind.icon(), ink, tint)
            .footer(footer)
            .on_close(Message::Respond(false))
            .close_on_scrim(false)
            .into()
    }
}

#[cfg(test)]
#[path = "../../../../tests/app/components/alert/mod.rs"]
mod tests;
