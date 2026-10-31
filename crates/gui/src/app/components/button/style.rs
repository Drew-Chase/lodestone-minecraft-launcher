//! The target appearance of a [`TextButton`] for each variant and status.

use crate::app::components::anim;
use crate::app::theme::color;
use iced::Color;
use iced_color_extension::rgb;

use super::{DANGER, Status, TextButton, Variant, Visual};

impl<'a, Message: Clone + 'a> TextButton<'a, Message> {
    /// The target appearance for the given status.
    pub(super) fn visual(&self, status: Status) -> Visual {
        if self.disabled || status == Status::Disabled {
            // The guide fades the whole button to 40% opacity.
            return match self.variant {
                Variant::Primary => Visual {
                    background: rgb!(34, 255, 132, 40%),
                    foreground: anim::fade(color::Primary::ON, 0.4),
                    border: rgb!(34, 255, 132, 12%),
                    lift: 0.0,
                    glow: 0.0,
                },
                Variant::Ghost => Visual {
                    background: rgb!(255, 255, 255, 2%),
                    foreground: color::Text::DISABLED,
                    border: rgb!(255, 255, 255, 3%),
                    lift: 0.0,
                    glow: 0.0,
                },
                Variant::Danger => Visual {
                    background: anim::fade(DANGER, 0.04),
                    foreground: color::Text::DISABLED,
                    border: anim::fade(DANGER, 0.08),
                    lift: 0.0,
                    glow: 0.0,
                },
                Variant::Destructive => Visual {
                    background: anim::fade(DANGER, 0.4),
                    foreground: color::Text::DISABLED,
                    border: anim::fade(DANGER, 0.12),
                    lift: 0.0,
                    glow: 0.0,
                },
                Variant::Tab => Visual {
                    background: Color::TRANSPARENT,
                    foreground: color::Text::DISABLED,
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
                    foreground: color::Primary::ON,
                    border: rgb!(34, 255, 132, 50%),
                    lift: 1.0,
                    glow: 1.0,
                },
                // Press — translateY(0); the glow hugs the surface.
                Status::Pressed => Visual {
                    background: color::Primary::DIM,
                    foreground: color::Primary::ON,
                    border: rgb!(34, 255, 132, 30%),
                    lift: 0.0,
                    glow: 0.4,
                },
                _ => Visual {
                    background: color::Primary::DEFAULT,
                    foreground: color::Primary::ON,
                    border: rgb!(34, 255, 132, 30%),
                    lift: 0.0,
                    glow: 0.75,
                },
            },
            Variant::Ghost => match status {
                // Hover — white 8%, emphasized border (§07 spec).
                Status::Hovered => Visual {
                    background: rgb!(255, 255, 255, 8%),
                    foreground: color::Text::HEADLINE,
                    border: color::Surface::LINE_STRONG,
                    lift: 1.0,
                    glow: 0.0,
                },
                Status::Pressed => Visual {
                    background: rgb!(255, 255, 255, 6%),
                    foreground: color::Text::HEADLINE,
                    border: color::Surface::LINE_STRONG,
                    lift: 0.0,
                    glow: 0.0,
                },
                _ => Visual {
                    background: rgb!(255, 255, 255, 4%),
                    foreground: color::Text::DEFAULT,
                    border: color::Surface::LINE,
                    lift: 0.0,
                    glow: 0.0,
                },
            },
            // Destructive — red ink on a red tint at rest; hover turns the
            // fill solid red with headline ink.
            Variant::Danger => match status {
                Status::Hovered => Visual {
                    background: DANGER,
                    foreground: color::Text::HEADLINE,
                    border: DANGER,
                    lift: 1.0,
                    glow: 0.0,
                },
                Status::Pressed => Visual {
                    background: anim::fade(DANGER, 0.8),
                    foreground: color::Text::HEADLINE,
                    border: DANGER,
                    lift: 0.0,
                    glow: 0.0,
                },
                _ => Visual {
                    background: anim::fade(DANGER, 0.12),
                    foreground: DANGER,
                    border: anim::fade(DANGER, 0.3),
                    lift: 0.0,
                    glow: 0.0,
                },
            },
            // Destructive — the solid red fill at rest, brightened on hover.
            Variant::Destructive => match status {
                Status::Hovered => Visual {
                    background: DANGER,
                    foreground: color::Text::HEADLINE,
                    border: DANGER,
                    lift: 1.0,
                    glow: 0.0,
                },
                Status::Pressed => Visual {
                    background: anim::fade(DANGER, 0.8),
                    foreground: color::Text::HEADLINE,
                    border: DANGER,
                    lift: 0.0,
                    glow: 0.0,
                },
                _ => Visual {
                    background: DANGER,
                    foreground: color::Text::HEADLINE,
                    border: DANGER,
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
                    foreground: color::Text::HEADLINE,
                    border: Color::TRANSPARENT,
                    lift: 0.0,
                    glow: 0.0,
                },
                Status::Pressed => Visual {
                    background: rgb!(255, 255, 255, 5%),
                    foreground: color::Text::HEADLINE,
                    border: Color::TRANSPARENT,
                    lift: 0.0,
                    glow: 0.0,
                },
                _ => Visual {
                    background: Color::TRANSPARENT,
                    foreground: color::Text::MUTED,
                    border: Color::TRANSPARENT,
                    lift: 0.0,
                    glow: 0.0,
                },
            },
        }
    }
}
