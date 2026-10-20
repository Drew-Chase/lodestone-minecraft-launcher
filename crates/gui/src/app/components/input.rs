//! Input styles — the style guide's `.input` field (§07), as a
//! [`iced::widget::text_input`] stylesheet.
//!
//! ```text
//! use crate::app::components::input;
//!
//! text_input("Search 48,000 mods…", value)
//!     .on_input(Message::SearchChanged)
//!     .style(input::style)
//! ```
//!
//! States mirror the guide: white 4% fill with a hairline border at rest;
//! white 6% and `--line-strong` on hover; focus draws the green ring —
//! `border-color: rgba(34,255,132,.5)` over a green 3% fill. Iced's
//! `text_input::Style` has no outer-glow slot, so the guide's
//! `box-shadow: 0 0 0 3px rgba(34,255,132,.1)` halo is approximated by the
//! border alone.

use crate::app::theme::color;
use iced::widget::text_input;
use iced::{Background, Border, Color, Theme};
use iced_color_extension::rgb;

/// `.input` — 12px radius, white 4% fill, hairline border.
pub fn style(_theme: &Theme, status: text_input::Status) -> text_input::Style {
    let (background, border_color, value, placeholder) = match status {
        // Focus — green ring over a green 3% fill (§07 `.input:focus`).
        text_input::Status::Focused { .. } => (
            rgb!(34, 255, 132, 3%),
            rgb!(34, 255, 132, 50%),
            color::Text::DEFAULT,
            color::Text::PLACEHOLDER,
        ),
        text_input::Status::Hovered => (
            rgb!(255, 255, 255, 6%),
            color::Surface::LINE_STRONG,
            color::Text::DEFAULT,
            color::Text::PLACEHOLDER,
        ),
        text_input::Status::Disabled => (
            rgb!(255, 255, 255, 2%),
            color::Surface::LINE,
            color::Text::DISABLED,
            color::Text::DISABLED,
        ),
        text_input::Status::Active => (
            rgb!(255, 255, 255, 4%),
            color::Surface::LINE,
            color::Text::DEFAULT,
            color::Text::PLACEHOLDER,
        ),
    };

    text_input::Style {
        background: Background::Color(background),
        border: Border {
            radius: 12.0.into(),
            width: 1.0,
            color: border_color,
        },
        icon: color::Text::PLACEHOLDER,
        placeholder,
        value,
        selection: selection_color(),
    }
}

/// Text selection — Lode Green at 25%, readable under any ink.
pub fn selection_color() -> Color {
    rgb!(34, 255, 132, 25%)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn focus_ring_is_green() {
        let focused = style(&Theme::Dark, text_input::Status::Focused { is_hovered: false });
        assert_eq!(focused.border.color, rgb!(34, 255, 132, 50%));
        assert_eq!(focused.background, Background::Color(rgb!(34, 255, 132, 3%)));
    }

    #[test]
    fn disabled_uses_disabled_ink() {
        let disabled = style(&Theme::Dark, text_input::Status::Disabled);
        assert_eq!(disabled.value, color::Text::DISABLED);
        assert_eq!(disabled.placeholder, color::Text::DISABLED);
    }

    #[test]
    fn idle_matches_the_spec() {
        let active = style(&Theme::Dark, text_input::Status::Active);
        assert_eq!(active.border.radius, 12.0.into());
        assert_eq!(active.background, Background::Color(rgb!(255, 255, 255, 4%)));
    }
}
