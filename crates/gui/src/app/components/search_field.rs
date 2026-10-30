//! The search field at the top or bottom of a searchable menu: the text it holds, the caret in
//! that text, and how both are drawn.
//!
//! The caret blinks the way iced's own text inputs do. It shows for half of each blink and hides
//! for the other half, and it shows again from the start whenever it moves or the text changes.

use std::time::{Duration, Instant};

use aurora_leaf::Icon;
use iced::advanced::svg::Renderer as _;
use iced::advanced::svg::{Handle, Svg};
use iced::advanced::text::{self, Paragraph as _, Renderer as _};
use iced::advanced::{Renderer as _, renderer};
use iced::border::Radius;
use iced::keyboard;
use iced::keyboard::key::{Code, Physical};
use iced::{Background, Border, Pixels, Point, Radians, Rectangle, Renderer as BaseRenderer, Size, alignment};
use iced_color_extension::rgb;

use super::anim;
use crate::app::lang;
use crate::app::theme::{color, fonts};

/// Each half of the caret's blink, in milliseconds: shown for one, hidden for the next.
const BLINK_MILLIS: u128 = 500;
/// The text's size and leading, as the field sets them.
const SIZE: f32 = 13.0;
const LEADING: f32 = 1.4;
/// The caret is one pixel wide and as tall as a line of the text.
const CARET_WIDTH: f32 = 1.0;
const CARET_HEIGHT: f32 = SIZE * LEADING;
/// The text starts past the search glyph, and keeps a gap from the field's right edge.
const TEXT_LEFT: f32 = 32.0;
const TEXT_RIGHT: f32 = 10.0;

/// The text a search field holds, and the caret's place in it.
///
/// The caret is a byte offset that always sits on a character boundary, so every edit and move
/// leaves the text whole.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub(crate) struct Query {
    text: String,
    caret: usize,
}

impl Query {
    /// The text, in full.
    pub(crate) fn text(&self) -> &str {
        &self.text
    }

    /// The text before the caret.
    pub(crate) fn before_caret(&self) -> &str {
        &self.text[..self.caret]
    }

    pub(crate) fn is_empty(&self) -> bool {
        self.text.is_empty()
    }

    /// Empties the field, with the caret back at the start.
    pub(crate) fn clear(&mut self) {
        self.text.clear();
        self.caret = 0;
    }

    /// Types `typed` at the caret, which moves past it. Control characters are dropped. Returns
    /// whether any text was added.
    pub(crate) fn insert(&mut self, typed: &str) -> bool {
        let clean: String = typed.chars().filter(|character| !character.is_control()).collect();
        if clean.is_empty() {
            return false;
        }
        self.text.insert_str(self.caret, &clean);
        self.caret += clean.len();
        true
    }

    /// Deletes the character before the caret. Returns whether anything was deleted.
    pub(crate) fn backspace(&mut self) -> bool {
        let Some((start, _)) = self.text[..self.caret].char_indices().next_back() else {
            return false;
        };
        self.text.replace_range(start..self.caret, "");
        self.caret = start;
        true
    }

    /// Deletes the character after the caret. Returns whether anything was deleted.
    pub(crate) fn delete(&mut self) -> bool {
        let Some(next) = self.text[self.caret..].chars().next() else {
            return false;
        };
        self.text.replace_range(self.caret..self.caret + next.len_utf8(), "");
        true
    }

    /// Moves the caret one character left, stopping at the start.
    pub(crate) fn left(&mut self) {
        if let Some((start, _)) = self.text[..self.caret].char_indices().next_back() {
            self.caret = start;
        }
    }

    /// Moves the caret one character right, stopping at the end.
    pub(crate) fn right(&mut self) {
        if let Some(next) = self.text[self.caret..].chars().next() {
            self.caret += next.len_utf8();
        }
    }

    pub(crate) fn home(&mut self) {
        self.caret = 0;
    }

    pub(crate) fn end(&mut self) {
        self.caret = self.text.len();
    }
}

/// The text a key press types into a field, if any. The key's own text is used when it has some.
/// Without text, a character key types its character, and a keypad key types what it prints. With
/// NumLock off the keypad sends navigation keys instead, so the keypad's text is read from the
/// key's position. Shortcuts with Ctrl, Alt or the logo key type nothing, and so do control
/// characters such as the one a Tab press carries.
pub(crate) fn typed_text<'a>(key: &'a keyboard::Key, physical: Physical, modifiers: keyboard::Modifiers, text: Option<&'a str>) -> Option<&'a str> {
    if modifiers.control() || modifiers.alt() || modifiers.logo() {
        return None;
    }
    let typed = match text {
        Some(typed) if !typed.is_empty() => typed,
        _ => match key {
            keyboard::Key::Character(character) => character.as_str(),
            _ => keypad_text(physical)?,
        },
    };
    (!typed.chars().any(char::is_control)).then_some(typed)
}

/// The text a keypad key prints: its digit, or a dot for the decimal key. The decimal key types a
/// dot even on a layout that prints a comma there, since version names use dots.
fn keypad_text(physical: Physical) -> Option<&'static str> {
    let Physical::Code(code) = physical else {
        return None;
    };
    Some(match code {
        Code::Numpad0 => "0",
        Code::Numpad1 => "1",
        Code::Numpad2 => "2",
        Code::Numpad3 => "3",
        Code::Numpad4 => "4",
        Code::Numpad5 => "5",
        Code::Numpad6 => "6",
        Code::Numpad7 => "7",
        Code::Numpad8 => "8",
        Code::Numpad9 => "9",
        Code::NumpadDecimal => ".",
        _ => return None,
    })
}

/// Whether the caret is shown at `now`, for a caret that last moved at `since`.
pub(crate) fn caret_visible(since: Instant, now: Instant) -> bool {
    (elapsed_millis(since, now) / BLINK_MILLIS).is_multiple_of(2)
}

/// How long from `now` until the caret next flips between shown and hidden.
pub(crate) fn next_blink(since: Instant, now: Instant) -> Duration {
    let remaining = BLINK_MILLIS - elapsed_millis(since, now) % BLINK_MILLIS;
    Duration::from_millis(remaining as u64)
}

fn elapsed_millis(since: Instant, now: Instant) -> u128 {
    now.saturating_duration_since(since).as_millis()
}

/// Draws the field into `field`: the frame, the search glyph, the text and, while `caret_on`, the
/// caret. Everything fades with `alpha` as the menu opens and closes.
pub(crate) fn draw(renderer: &mut BaseRenderer, field: Rectangle, query: &Query, caret_on: bool, alpha: f32) {
    // The field is always the focus while the menu is open: a faint green edge.
    renderer.fill_quad(
        renderer::Quad {
            bounds: field,
            border: Border {
                radius: Radius::from(8.0),
                width: 1.0,
                color: anim::fade(rgb!(34, 255, 132, 35%), alpha),
            },
            ..renderer::Quad::default()
        },
        Background::Color(anim::fade(rgb!(255, 255, 255, 4%), alpha)),
    );

    let glyph = 14.0;
    let glyph_bounds = Rectangle::new(Point::new(field.x + 10.0, field.center_y() - glyph / 2.0), Size::new(glyph, glyph));
    renderer.draw_svg(
        Svg {
            handle: Handle::from_memory(Icon::lucide().search().as_bytes()),
            opacity: 1.0,
            rotation: Radians(0.0),
            color: Some(anim::fade(color::Text::MUTED, alpha)),
        },
        glyph_bounds,
        field,
    );

    // The text is clipped to its own box, so a long query slides under the glyph's edge rather
    // than over it.
    let area = Rectangle::new(
        Point::new(field.x + TEXT_LEFT, field.y),
        Size::new((field.width - TEXT_LEFT - TEXT_RIGHT).max(0.0), field.height),
    );
    // Once the text outgrows the box, it slides left so the caret stays inside the right edge.
    let caret_x = width_of(query.before_caret());
    let scroll = (caret_x + CARET_WIDTH - area.width).max(0.0);
    let origin = Point::new(area.x - scroll, field.center_y());
    let (content, ink) = if query.is_empty() {
        (lang::t("common.search"), color::Text::PLACEHOLDER)
    } else {
        (query.text(), color::Text::DEFAULT)
    };

    renderer.with_layer(area, |renderer| {
        renderer.fill_text(
            text::Text {
                content: content.to_owned(),
                bounds: Size::new(f32::INFINITY, field.height),
                size: Pixels::from(SIZE),
                line_height: text::LineHeight::Relative(LEADING),
                font: fonts::Inter::MEDIUM,
                align_x: text::Alignment::Default,
                align_y: alignment::Vertical::Center,
                shaping: text::Shaping::Basic,
                wrapping: text::Wrapping::None,
            },
            origin,
            anim::fade(ink, alpha),
            area,
        );

        if caret_on {
            renderer.fill_quad(
                renderer::Quad {
                    bounds: Rectangle::new(
                        Point::new(area.x + caret_x - scroll, field.center_y() - CARET_HEIGHT / 2.0),
                        Size::new(CARET_WIDTH, CARET_HEIGHT),
                    ),
                    ..renderer::Quad::default()
                },
                Background::Color(anim::fade(color::Text::DEFAULT, alpha)),
            );
        }
    });
}

/// The width of `content` set the way the field sets its text, on one line that never wraps.
fn width_of(content: &str) -> f32 {
    if content.is_empty() {
        return 0.0;
    }
    <BaseRenderer as text::Renderer>::Paragraph::with_text(text::Text {
        content,
        bounds: Size::new(f32::INFINITY, SIZE * LEADING),
        size: Pixels::from(SIZE),
        line_height: text::LineHeight::Relative(LEADING),
        font: fonts::Inter::MEDIUM,
        align_x: text::Alignment::Default,
        align_y: alignment::Vertical::Center,
        shaping: text::Shaping::Basic,
        wrapping: text::Wrapping::None,
    })
    .min_width()
}

#[cfg(test)]
#[path = "../../../tests/app/components/search_field.rs"]
mod tests;
