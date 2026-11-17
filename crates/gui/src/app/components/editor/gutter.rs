//! The line numbers beside the editor's text.
//!
//! iced's text editor draws no gutter, so this widget draws one next to it. It is handed how far the text has
//! scrolled, and draws only the numbers of the lines on screen, each on the same line height as the text, so a number
//! always sits beside its line.

use iced::advanced::layout::{Limits, Node};
use iced::advanced::renderer;
use iced::advanced::text::{self, Renderer as _};
use iced::advanced::widget::{Tree, tree};
use iced::advanced::{Layout, Widget};
use iced::mouse::Cursor;
use iced::{Element, Length, Pixels, Point, Rectangle, Renderer as BaseRenderer, Size, Theme, alignment};

use crate::app::theme::{color, fonts};

/// The room either side of the numbers.
const INSET: f32 = 10.0;

/// The gutter of an editor. Built with [`Gutter::new`].
pub struct Gutter {
    /// How many lines the text has.
    lines: usize,
    /// The line the caret is on, whose number is drawn brighter.
    current: usize,
    /// How far down the text has scrolled.
    scroll: f32,
    /// The room above the first line, which the text keeps too.
    top: f32,
    line_height: f32,
    size: f32,
    /// How wide one digit is.
    advance: f32,
}

impl Gutter {
    /// A gutter for a text of `lines` lines scrolled down by `scroll`, with the caret on line `current`.
    pub fn new(lines: usize, current: usize, scroll: f32) -> Self {
        Self {
            lines: lines.max(1),
            current,
            scroll,
            top: 0.0,
            line_height: 20.0,
            size: 13.0,
            advance: 7.8,
        }
    }

    /// Sets the metrics of the text beside the gutter: the room above its first line, its line height, its size and
    /// the width of one of its digits.
    pub fn metrics(mut self, top: f32, line_height: f32, size: f32, advance: f32) -> Self {
        self.top = top;
        self.line_height = line_height;
        self.size = size;
        self.advance = advance;
        self
    }

    /// How wide the gutter is: room for the digits of the last line's number, and never fewer than two.
    fn width(&self) -> f32 {
        width_for(self.lines, self.advance)
    }
}

/// The width of a gutter that numbers `lines` lines in digits `advance` wide.
pub fn width_for(lines: usize, advance: f32) -> f32 {
    let digits = lines.max(1).to_string().len().max(2);
    digits as f32 * advance + INSET * 2.0
}

/// The lines whose numbers show in a gutter `height` tall: the first one on screen, and the one after the last.
fn visible(scroll: f32, top: f32, line_height: f32, height: f32, lines: usize) -> std::ops::Range<usize> {
    let first = ((scroll - top) / line_height).floor().max(0.0) as usize;
    let last = ((scroll - top + height) / line_height).ceil().max(0.0) as usize;
    first.min(lines)..last.min(lines)
}

impl<Message> Widget<Message, Theme, BaseRenderer> for Gutter {
    fn tag(&self) -> tree::Tag {
        tree::Tag::stateless()
    }

    fn size(&self) -> Size<Length> {
        Size {
            width: Length::Fixed(self.width()),
            height: Length::Fill,
        }
    }

    fn layout(&mut self, _tree: &mut Tree, _renderer: &BaseRenderer, limits: &Limits) -> Node {
        Node::new(
            limits
                .width(self.width())
                .height(Length::Fill)
                .resolve(self.width(), Length::Fill, Size::ZERO),
        )
    }

    fn draw(
        &self,
        _tree: &Tree,
        renderer: &mut BaseRenderer,
        _theme: &Theme,
        _style: &renderer::Style,
        layout: Layout<'_>,
        _cursor: Cursor,
        _viewport: &Rectangle,
    ) {
        let bounds = layout.bounds();
        let right = bounds.x + bounds.width - INSET;
        for line in visible(self.scroll, self.top, self.line_height, bounds.height, self.lines) {
            let y = bounds.y + self.top + line as f32 * self.line_height - self.scroll;
            let ink = if line == self.current {
                color::text_default()
            } else {
                color::text_disabled()
            };
            renderer.fill_text(
                text::Text {
                    content: (line + 1).to_string(),
                    bounds: Size::new(bounds.width, self.line_height),
                    size: Pixels::from(self.size),
                    line_height: text::LineHeight::Absolute(Pixels::from(self.line_height)),
                    font: fonts::JetBrainsMono::REGULAR,
                    align_x: text::Alignment::Right,
                    align_y: alignment::Vertical::Top,
                    shaping: text::Shaping::Basic,
                    wrapping: text::Wrapping::None,
                },
                Point::new(right, y),
                ink,
                bounds,
            );
        }
    }
}

impl<'a, Message: 'a> From<Gutter> for Element<'a, Message> {
    fn from(gutter: Gutter) -> Self {
        Element::new(gutter)
    }
}

#[cfg(test)]
#[path = "../../../../tests/app/components/editor/gutter.rs"]
mod tests;
