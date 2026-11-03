//! The target appearance of a [`TextButton`] for each variant and status.

use crate::app::components::anim;
use crate::app::theme::color;
use iced::Color;
use iced_color_extension::rgb;

use super::{Status, TextButton, Variant, Visual, danger};

impl<'a, Message: Clone + 'a> TextButton<'a, Message> {
    /// The target appearance for the given status.
    pub(super) fn visual(&self, status: Status) -> Visual {
        if self.disabled || status == Status::Disabled {
            // The guide fades the whole button to 40% opacity.
            return match self.variant {
                Variant::Primary => Visual {
                    background: rgb!(34, 255, 132, 40%),
                    foreground: anim::fade(color::primary_on(), 0.4),
                    border: rgb!(34, 255, 132, 12%),
                    lift: 0.0,
                    glow: 0.0,
                },
                Variant::Ghost => Visual {
                    background: rgb!(255, 255, 255, 2%),
                    foreground: color::text_disabled(),
                    border: rgb!(255, 255, 255, 3%),
                    lift: 0.0,
                    glow: 0.0,
                },
                Variant::Danger => Visual {
                    background: anim::fade(danger(), 0.04),
                    foreground: color::text_disabled(),
                    border: anim::fade(danger(), 0.08),
                    lift: 0.0,
                    glow: 0.0,
                },
                Variant::Destructive => Visual {
                    background: anim::fade(danger(), 0.4),
                    foreground: color::text_disabled(),
                    border: anim::fade(danger(), 0.12),
                    lift: 0.0,
                    glow: 0.0,
                },
                Variant::Tab => Visual {
                    background: Color::TRANSPARENT,
                    foreground: color::text_disabled(),
                    border: Color::TRANSPARENT,
                    lift: 0.0,
                    glow: 0.0,
                },
            };
        }

        match self.variant {
            Variant::Primary => match status {
                // Hover — `brightness(1.06)`, lift, stronger glow (§07 spec).
                Status::Hovered => Visual {
                    background: iced::color!(0x2AFF8C),
                    foreground: color::primary_on(),
                    border: rgb!(34, 255, 132, 50%),
                    lift: 1.0,
                    glow: 1.0,
                },
                // Press — translateY(0); the glow hugs the surface.
                Status::Pressed => Visual {
                    background: color::primary_dim(),
                    foreground: color::primary_on(),
                    border: rgb!(34, 255, 132, 30%),
                    lift: 0.0,
                    glow: 0.4,
                },
                _ => Visual {
                    background: color::primary(),
                    foreground: color::primary_on(),
                    border: rgb!(34, 255, 132, 30%),
                    lift: 0.0,
                    glow: 0.75,
                },
            },
            Variant::Ghost => match status {
                // Hover — white 8%, emphasized border (§07 spec).
                Status::Hovered => Visual {
                    background: rgb!(255, 255, 255, 8%),
                    foreground: color::text_headline(),
                    border: color::surface_line_strong(),
                    lift: 1.0,
                    glow: 0.0,
                },
                Status::Pressed => Visual {
                    background: rgb!(255, 255, 255, 6%),
                    foreground: color::text_headline(),
                    border: color::surface_line_strong(),
                    lift: 0.0,
                    glow: 0.0,
                },
                _ => Visual {
                    background: rgb!(255, 255, 255, 4%),
                    foreground: color::text_default(),
                    border: color::surface_line(),
                    lift: 0.0,
                    glow: 0.0,
                },
            },
            // Destructive — red ink on a red tint at rest; hover turns the
            // fill solid red with headline ink.
            Variant::Danger => match status {
                Status::Hovered => Visual {
                    background: danger(),
                    foreground: color::text_headline(),
                    border: danger(),
                    lift: 1.0,
                    glow: 0.0,
                },
                Status::Pressed => Visual {
                    background: anim::fade(danger(), 0.8),
                    foreground: color::text_headline(),
                    border: danger(),
                    lift: 0.0,
                    glow: 0.0,
                },
                _ => Visual {
                    background: anim::fade(danger(), 0.12),
                    foreground: danger(),
                    border: anim::fade(danger(), 0.3),
                    lift: 0.0,
                    glow: 0.0,
                },
            },
            // Destructive — the solid red fill at rest, brightened on hover.
            Variant::Destructive => match status {
                Status::Hovered => Visual {
                    background: danger(),
                    foreground: color::text_headline(),
                    border: danger(),
                    lift: 1.0,
                    glow: 0.0,
                },
                Status::Pressed => Visual {
                    background: anim::fade(danger(), 0.8),
                    foreground: color::text_headline(),
                    border: danger(),
                    lift: 0.0,
                    glow: 0.0,
                },
                _ => Visual {
                    background: danger(),
                    foreground: color::text_headline(),
                    border: danger(),
                    lift: 0.0,
                    glow: 0.0,
                },
            },
            // Tabs are chrome-free: only the label (and the page's
            // underline) changes. Hover brightens the ink and leaves a
            // barely-there tint so the row still feels alive.
            Variant::Tab => match status {
                Status::Hovered => Visual {
                    background: rgb!(255, 255, 255, 3%),
                    foreground: color::text_headline(),
                    border: Color::TRANSPARENT,
                    lift: 0.0,
                    glow: 0.0,
                },
                Status::Pressed => Visual {
                    background: rgb!(255, 255, 255, 5%),
                    foreground: color::text_headline(),
                    border: Color::TRANSPARENT,
                    lift: 0.0,
                    glow: 0.0,
                },
                _ => Visual {
                    background: Color::TRANSPARENT,
                    foreground: color::text_muted(),
                    border: Color::TRANSPARENT,
                    lift: 0.0,
                    glow: 0.0,
                },
            },
        }
    }
}
