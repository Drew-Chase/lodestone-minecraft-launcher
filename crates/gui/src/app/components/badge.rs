//! Badges — the uppercase mono chips that tag loaders, categories and status.
//!
//! ```text
//! Badge::new("FABRIC", ink, fill).compact()
//! Badge::new("FABRIC", ink, fill).compact().with_loader(Loader::Fabric)
//! Badge::new("LAUNCHED", ink, fill).with_pulse(true)
//! ```

use crate::app::pages::library::instance::Loader;
use crate::app::theme::{color, fonts};
use iced::widget::{Text, container, row};
use iced::{Background, Border, Color, Element, Padding, alignment};

use super::loader_icon::loader_icon;
use super::status::PulseDot;

/// A rounded mono label on a tinted fill.
pub struct Badge {
    label: String,
    ink: Color,
    fill: Color,
    compact: bool,
    pulse: Option<bool>,
    loader: Option<Loader>,
}

impl Badge {
    /// A standard badge: 11px label, 4px vertical padding.
    pub fn new(label: impl Into<String>, ink: Color, fill: Color) -> Self {
        Self {
            label: label.into(),
            ink,
            fill,
            compact: false,
            pulse: None,
            loader: None,
        }
    }

    /// The smaller size used by list chips: 10px label, 3px vertical padding.
    pub fn compact(mut self) -> Self {
        self.compact = true;
        self
    }

    /// Adds a status dot before the label, pulsing when `pulsing` is set.
    pub fn with_pulse(mut self, pulsing: bool) -> Self {
        self.pulse = Some(pulsing);
        self
    }

    /// Draws the loader's mark before the label: 12px in a compact badge, 13px otherwise.
    pub fn with_loader(mut self, loader: Loader) -> Self {
        self.loader = Some(loader);
        self
    }
}

/// The ink and tinted fill a mod loader is drawn with.
pub fn loader_palette(loader: Loader) -> (Color, Color) {
    match loader {
        Loader::Vanilla => (color::Primary::DEFAULT, color::Primary::TINT),
        Loader::Fabric => (color::Accents::VIOLET_LIGHT, color::Accents::VIOLET_TINT),
        Loader::Forge => (color::Accents::AMBER, color::Accents::AMBER_TINT),
        Loader::Quilt => (color::Accents::CYAN, color::Accents::CYAN_TINT),
        Loader::NeoForge => (color::Accents::PINK, color::Accents::PINK_TINT),
    }
}

impl<'a, Message: 'a> From<Badge> for Element<'a, Message> {
    fn from(badge: Badge) -> Self {
        let Badge {
            label,
            ink,
            fill,
            compact,
            pulse,
            loader,
        } = badge;
        let (size, vertical) = if compact { (10.0, 3.0) } else { (11.0, 4.0) };
        let mark = if compact { 12.0 } else { 13.0 };
        let text = Text::new(label).size(size).font(fonts::JetBrainsMono::SEMI_BOLD).color(ink);
        let content: Element<'a, Message> = match (loader, pulse) {
            (None, None) => text.into(),
            (loader, pulse) => {
                let mut parts: Vec<Element<'a, Message>> = Vec::with_capacity(3);
                if let Some(loader) = loader {
                    parts.push(loader_icon(loader, mark));
                }
                if let Some(pulsing) = pulse {
                    parts.push(PulseDot::with_color(ink).size(5.0).pulsing(pulsing).into());
                }
                parts.push(text.into());
                row(parts).spacing(6).align_y(alignment::Vertical::Center).into()
            }
        };

        container(content)
            .padding(Padding::new(0.0).left(8).right(8).top(vertical).bottom(vertical))
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
}
