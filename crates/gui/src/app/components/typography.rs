//! Typography — the type scale from the Lodestone style guide (`§04`).
//!
//! Every style is a plain function returning an [`iced::widget::Text`], so all
//! of iced's builder methods still chain after it:
//!
//! ```text
//! use crate::{display, h1, body, small, eyebrow, code};
//!
//! display!("Play anything.")
//! h1!("Your Library")
//! body!("Hello world").color(color!(0xff00ff))
//! small!("Last played 2 hours ago").align_x(iced::widget::text::Alignment::Right)
//! eyebrow!("Instances · 12") // rendered as "INSTANCES · 12"
//! code!("[14:02:11] INFO Loading 412 mods")
//! ```
//!
//! The guide's per-style ink colors are applied as defaults and can be
//! overridden by chaining [`iced::widget::Text::color`].
//!
//! Limitations imposed by iced (no letter-spacing / text-transform APIs):
//! the guide's tracking values (−4%…−1%, eyebrow +8%) are not applied, and
//! `eyebrow` uppercases its content at runtime. `display` is fixed at 72px —
//! chain `.size(40)` where that is too large.

use crate::app::theme::{color, fonts};
use iced::widget::Text;
use iced::widget::text::IntoFragment;
use iced::{Color, Font};

/// Applies one step of the type scale: font, size, and optional ink color.
/// Line-height is added per style, since the guide does not define one for
/// every row.
fn text<'a>(content: impl IntoFragment<'a>, size: f32, font: Font, ink: Option<Color>) -> Text<'a> {
    let styled = Text::new(content).font(font).size(size);
    if let Some(ink) = ink { styled.color(ink) } else { styled }
}

/// Display — 72 / 0.95 · 800. Hero moments.
pub fn display<'a>(content: impl IntoFragment<'a>) -> Text<'a> {
    text(content, 72.0, fonts::Inter::EXTRA_BOLD, Some(color::Text::HEADLINE)).line_height(0.95)
}

/// H1 — 40 / 1.1 · 700.
pub fn h1<'a>(content: impl IntoFragment<'a>) -> Text<'a> {
    text(content, 40.0, fonts::Inter::BOLD, Some(color::Text::HEADLINE)).line_height(1.1)
}

/// H2 — 28 / 1.2 · 700.
pub fn h2<'a>(content: impl IntoFragment<'a>) -> Text<'a> {
    text(content, 28.0, fonts::Inter::BOLD, Some(color::Text::HEADLINE)).line_height(1.2)
}

/// H3 — 18 / 1.3 · 600.
pub fn h3<'a>(content: impl IntoFragment<'a>) -> Text<'a> {
    text(content, 18.0, fonts::Inter::SEMI_BOLD, Some(color::Text::HEADLINE)).line_height(1.3)
}

/// Body — 14 / 1.55 · 400. Inherits the app-wide default ink (`--ink-1`).
pub fn body<'a>(content: impl IntoFragment<'a>) -> Text<'a> {
    text(content, 14.0, fonts::Inter::REGULAR, None).line_height(1.55)
}

/// Small / UI — 13 / 1.4 · 500 · `--ink-2`.
pub fn small<'a>(content: impl IntoFragment<'a>) -> Text<'a> {
    text(content, 13.0, fonts::Inter::MEDIUM, Some(color::Text::MUTED)).line_height(1.4)
}

/// Eyebrow — 11 · mono 600 · caps · `--ink-3`. The content is uppercased at
/// runtime, since iced has no text-transform.
pub fn eyebrow<'a>(content: impl IntoFragment<'a>) -> Text<'a> {
    text(
        content.into_fragment().to_uppercase(),
        11.0,
        fonts::JetBrainsMono::SEMI_BOLD,
        Some(color::Text::PLACEHOLDER),
    )
}

/// Code / Log — 12 / 1.6 · mono 400 · `--ink-2`.
pub fn code<'a>(content: impl IntoFragment<'a>) -> Text<'a> {
    text(content, 12.0, fonts::JetBrainsMono::REGULAR, Some(color::Text::MUTED)).line_height(1.6)
}

/// Display — 72 / 0.95 · 800.
#[macro_export]
macro_rules! display {
    ($content:expr $(,)?) => {
        $crate::app::components::typography::display($content)
    };
}

/// H1 — 40 / 1.1 · 700.
#[macro_export]
macro_rules! h1 {
    ($content:expr $(,)?) => {
        $crate::app::components::typography::h1($content)
    };
}

/// H2 — 28 / 1.2 · 700.
#[macro_export]
macro_rules! h2 {
    ($content:expr $(,)?) => {
        $crate::app::components::typography::h2($content)
    };
}

/// H3 — 18 / 1.3 · 600.
#[macro_export]
macro_rules! h3 {
    ($content:expr $(,)?) => {
        $crate::app::components::typography::h3($content)
    };
}

/// Body — 14 / 1.55 · 400.
#[macro_export]
macro_rules! body {
    ($content:expr $(,)?) => {
        $crate::app::components::typography::body($content)
    };
}

/// Small / UI — 13 / 1.4 · 500.
#[macro_export]
macro_rules! small {
    ($content:expr $(,)?) => {
        $crate::app::components::typography::small($content)
    };
}

/// Eyebrow — 11 · mono 600 · caps.
#[macro_export]
macro_rules! eyebrow {
    ($content:expr $(,)?) => {
        $crate::app::components::typography::eyebrow($content)
    };
}

/// Code / Log — 12 / 1.6 · mono 400.
#[macro_export]
macro_rules! code {
    ($content:expr $(,)?) => {
        $crate::app::components::typography::code($content)
    };
}

#[cfg(test)]
#[path = "../../../tests/app/components/typography.rs"]
mod tests;
