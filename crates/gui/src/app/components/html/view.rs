//! A [`Document`] drawn as the launcher's elements. Text is rich text in the page's fonts, and an author's colour is
//! kept only where it reads against the panel. Pictures come from the ones the page has loaded and show a placeholder
//! until then. A press on a link or a linked picture, or on a spoiler's toggle, sends the message the caller names.

use std::collections::{HashMap, HashSet};

use aurora_leaf::Icon;
use iced::widget::text::{Span as TextSpan, Wrapping};
use iced::widget::{Row, Space, column, container, image, rich_text, row, rule, span, stack, text};
use iced::{Background, Border, Color, ContentFit, Element, Fill, Font, Length, Padding, Theme, alignment, font};

use super::blocks::VIDEO_WIDTH;
use super::css::{self, TEXT_SIZE};
use super::{Align, Block, Document, Picture, Span};
use crate::app::components::TextButton;
use crate::app::components::button::Variant;
use crate::app::components::panel;
use crate::app::components::tile::tile;
use crate::app::lang;
use crate::app::theme::{color, fonts, utility};

/// The gap between blocks: iced's markdown spacing at the page's text size.
const GAP: f32 = 12.0;
/// The gap between the items of a list.
const ITEM_GAP: f32 = 6.0;
/// The gap between a list item's marker and its content.
const MARKER_GAP: f32 = 6.0;
/// The gap between pictures in a row, across and down.
const PICTURE_GAP: f32 = 8.0;
/// The side of the placeholder for a picture with no size and no alt text.
const PLACEHOLDER: f32 = 48.0;
/// The corner radius of placeholders, code and the tiles around linked pictures.
const RADIUS: f32 = 8.0;
/// How far a spoiler's content sits in from its toggle.
const INDENT: f32 = 14.0;
/// The size of code blocks.
const CODE_SIZE: f32 = 12.0;
/// The size of code inside text, against the text around it.
const INLINE_CODE_SCALE: f32 = 0.9;
/// The size of a video's play mark.
const PLAY_SIZE: f32 = 22.0;
/// The bullets of lists, by how deep the list sits in other lists.
const BULLETS: [&str; 3] = ["•", "◦", "▪"];

/// What the view needs besides the document: the pictures the page has loaded, by address; the spoilers the reader has
/// opened; and the messages a link and a spoiler toggle send.
pub struct Viewer<'a, Message> {
    pub pictures: &'a HashMap<String, image::Handle>,
    pub open: &'a HashSet<usize>,
    pub on_link: fn(String) -> Message,
    pub on_toggle: fn(usize) -> Message,
}

/// The document's blocks in a column.
pub fn view<'a, Message: Clone + 'a>(document: &'a Document, viewer: &Viewer<'a, Message>) -> Element<'a, Message> {
    let drawer = Drawer {
        document,
        pictures: viewer.pictures,
        open: viewer.open,
        on_link: viewer.on_link,
        on_toggle: viewer.on_toggle,
        // The panel the document sits in is close to `--bg-2`, and a little darker, so a colour that reads on it reads on
        // the panel too. It is read on each draw, so a change of surface theme applies at once.
        background: color::surface_bg2(),
    };
    drawer.blocks(&document.blocks, 0)
}

/// What one draw of a document reads. It is copied into the drawing of each block.
struct Drawer<'a, Message> {
    document: &'a Document,
    pictures: &'a HashMap<String, image::Handle>,
    open: &'a HashSet<usize>,
    on_link: fn(String) -> Message,
    on_toggle: fn(usize) -> Message,
    /// The colour authors' colours are checked against.
    background: Color,
}

impl<Message> Clone for Drawer<'_, Message> {
    fn clone(&self) -> Self {
        *self
    }
}

impl<Message> Copy for Drawer<'_, Message> {}

impl<'a, Message: Clone + 'a> Drawer<'a, Message> {
    /// Blocks in a column. `depth` counts the lists around them, which picks their bullets.
    fn blocks(self, blocks: &'a [Block], depth: usize) -> Element<'a, Message> {
        column(blocks.iter().map(|block| self.block(block, depth)))
            .spacing(GAP)
            .width(Fill)
            .into()
    }

    fn block(self, block: &'a Block, depth: usize) -> Element<'a, Message> {
        match block {
            Block::Text { spans, heading, align } => self.text(spans, *heading, *align),
            Block::Pictures { pictures, align } => self.pictures(pictures, *align),
            Block::List { start, items } => self.list(*start, items, depth),
            Block::Quote(blocks) => self.quote(blocks, depth),
            Block::Code(code) => code_block(code),
            Block::Rule => panel::hairline(),
            Block::Table(rows) => self.table(rows, depth),
            Block::Spoiler { index, title, blocks } => self.spoiler(*index, title.as_deref(), blocks, depth),
        }
    }

    /// A paragraph or heading as one run of rich text. Long words, such as addresses, break where they must.
    fn text(self, spans: &'a [Span], heading: Option<u8>, align: Align) -> Element<'a, Message> {
        let size = heading.map_or(TEXT_SIZE, heading_size);
        let font = if heading.is_some() {
            fonts::Inter::SEMI_BOLD
        } else {
            fonts::Inter::REGULAR
        };
        let pieces: Vec<TextSpan<'a, usize, Font>> = spans.iter().map(|piece| self.span(piece, font, size)).collect();
        let links = &self.document.links;
        let on_link = self.on_link;
        rich_text(pieces)
            .size(size)
            .font(font)
            .color(color::text_default())
            .width(Fill)
            .align_x(horizontal(align))
            .wrapping(Wrapping::WordOrGlyph)
            .on_link_click(move |index: usize| on_link(links.get(index).cloned().unwrap_or_default()))
            .into()
    }

    /// One run of text in its style. A link is drawn in the accent. An author's colour is drawn only where it reads.
    fn span(self, piece: &'a Span, base: Font, size: f32) -> TextSpan<'a, usize, Font> {
        let style = piece.style;
        let mut font = if style.code { fonts::JetBrainsMono::REGULAR } else { base };
        if style.bold {
            font.weight = font::Weight::Bold;
        }
        if style.italic {
            font.style = font::Style::Italic;
        }
        let tint = if piece.link.is_some() {
            Some(color::primary())
        } else {
            style.color.filter(|tint| css::readable(*tint, self.background))
        };
        let size = style.size.unwrap_or(if style.code { size * INLINE_CODE_SCALE } else { size });
        span(piece.text.as_str())
            .font(font)
            .size(size)
            .color_maybe(tint)
            .link_maybe(piece.link)
            .underline(style.underline)
            .strikethrough(style.strike)
    }

    /// A row of pictures that wraps onto more lines, in its alignment. The group sizes to its pictures inside a
    /// full-width box, since a wrapping row lines up to its widest line rather than to the box.
    fn pictures(self, pictures: &'a [Picture], align: Align) -> Element<'a, Message> {
        let content: Element<'a, Message> = match pictures {
            [picture] => self.picture(picture),
            _ => Row::with_children(pictures.iter().map(|picture| self.picture(picture)))
                .spacing(PICTURE_GAP)
                .wrap()
                .vertical_spacing(PICTURE_GAP)
                .align_x(horizontal(align))
                .into(),
        };
        container(content).width(Fill).align_x(horizontal(align)).into()
    }

    /// A picture, pressable when it links somewhere.
    fn picture(self, picture: &'a Picture) -> Element<'a, Message> {
        let drawn = if picture.video { self.video(picture) } else { self.still(picture) };
        match picture.link.and_then(|index| self.document.links.get(index)) {
            Some(url) => tile(drawn, Length::Shrink, Length::Shrink, RADIUS, false, (self.on_link)(url.clone())),
            None => drawn,
        }
    }

    /// A picture at the width the HTML gave it, or at its own size. Either way the layout keeps it inside the page
    /// and keeps its shape. Until it loads, a placeholder stands in.
    fn still(self, picture: &'a Picture) -> Element<'a, Message> {
        let Some(handle) = self.pictures.get(&picture.url) else {
            return placeholder(picture);
        };
        let shown = image(handle.clone());
        match (picture.width, picture.height) {
            (Some(width), _) => shown.width(width).into(),
            (None, Some(height)) => shown.height(height).into(),
            (None, None) => shown.into(),
        }
    }

    /// A video as its thumbnail, cut to 16:9, with a play mark over it. Until the thumbnail loads, a panel says what
    /// the video is.
    fn video(self, picture: &'a Picture) -> Element<'a, Message> {
        let width = picture.width.unwrap_or(VIDEO_WIDTH);
        let height = width * 9.0 / 16.0;
        let thumbnail = self.pictures.get(&picture.url);
        let backdrop: Element<'a, Message> = match thumbnail {
            Some(handle) => image(handle.clone()).width(width).height(height).content_fit(ContentFit::Cover).into(),
            None => container(Space::new()).width(width).height(height).style(panel_style).into(),
        };
        let play = container(utility::icon(Icon::lucide().play(), PLAY_SIZE, Some((Color::WHITE, Color::WHITE))))
            .padding(14)
            .style(|_| container::Style {
                background: Some(Background::Color(Color::from_rgba(0.0, 0.0, 0.0, 0.6))),
                border: Border {
                    radius: 999.0.into(),
                    ..Border::default()
                },
                ..container::Style::default()
            });
        let mut mark = column![play].spacing(8).align_x(alignment::Horizontal::Center);
        if thumbnail.is_none() {
            mark = mark.push(
                text(lang::t("resource.video"))
                    .size(12)
                    .font(fonts::Inter::MEDIUM)
                    .color(color::text_muted()),
            );
        }
        stack![
            backdrop,
            container(mark)
                .width(width)
                .height(height)
                .align_x(alignment::Horizontal::Center)
                .align_y(alignment::Vertical::Center),
        ]
        .into()
    }

    /// A list: each item's marker beside its blocks. Numbers count up from the list's start, and bullets change with
    /// how deep the list sits.
    fn list(self, start: Option<u32>, items: &'a [Vec<Block>], depth: usize) -> Element<'a, Message> {
        let count = u32::try_from(items.len()).unwrap_or(u32::MAX);
        let marker_width = match start {
            Some(first) => 10.0 + 8.0 * digits(first.saturating_add(count)) as f32,
            None => 14.0,
        };
        column(items.iter().enumerate().map(|(position, item)| {
            let marker = match start {
                Some(first) => format!("{}.", first.saturating_add(u32::try_from(position).unwrap_or(u32::MAX))),
                None => BULLETS[depth.min(BULLETS.len() - 1)].to_owned(),
            };
            row![
                text(marker)
                    .size(TEXT_SIZE)
                    .font(fonts::Inter::REGULAR)
                    .color(color::text_muted())
                    .width(Length::Fixed(marker_width)),
                self.blocks(item, depth + 1),
            ]
            .spacing(MARKER_GAP)
            .into()
        }))
        .spacing(ITEM_GAP)
        .width(Fill)
        .into()
    }

    /// A quotation: a bar beside its blocks, laid out as iced's markdown lays out a quote.
    fn quote(self, blocks: &'a [Block], depth: usize) -> Element<'a, Message> {
        row![
            rule::vertical(3).style(|_| rule::Style {
                color: color::surface_line_strong(),
                radius: 2.0.into(),
                fill_mode: rule::FillMode::Full,
                snap: true,
            }),
            self.blocks(blocks, depth),
        ]
        .spacing(12)
        .height(Length::Shrink)
        .into()
    }

    /// A table: rows split by hairlines, each cell an equal share of the width. A short row is padded with empty
    /// cells, so the columns line up.
    fn table(self, rows: &'a [Vec<Vec<Block>>], depth: usize) -> Element<'a, Message> {
        let columns = rows.iter().map(Vec::len).max().unwrap_or_default();
        let lines: Vec<Element<'a, Message>> = rows
            .iter()
            .map(|cells| {
                let mut drawn: Vec<Element<'a, Message>> = cells
                    .iter()
                    .map(|cell| container(self.blocks(cell, depth)).width(Fill).padding(8).into())
                    .collect();
                drawn.extend((cells.len()..columns).map(|_| Space::new().width(Fill).into()));
                row(drawn).width(Fill).into()
            })
            .collect();
        panel::divided(lines)
    }

    /// A spoiler: its toggle, and its blocks beneath it while it is open. The toggle says the title the HTML gave it,
    /// or whether a press shows or hides it.
    fn spoiler(self, index: usize, title: Option<&'a str>, blocks: &'a [Block], depth: usize) -> Element<'a, Message> {
        let open = self.open.contains(&index);
        let label = match title {
            Some(title) => title.to_owned(),
            None if open => lang::t("resource.spoiler.hide").to_owned(),
            None => lang::t("resource.spoiler.show").to_owned(),
        };
        let icon = if open {
            Icon::lucide().chevron_down()
        } else {
            Icon::lucide().chevron_right()
        };
        let toggle: Element<'a, Message> = TextButton::labeled_icon(icon, label, Variant::Ghost)
            .on_press((self.on_toggle)(index))
            .into();
        if !open {
            return toggle;
        }
        column![toggle, container(self.blocks(blocks, depth)).padding(Padding::new(0.0).left(INDENT)),]
            .spacing(GAP)
            .width(Fill)
            .into()
    }
}

/// Preformatted text in a panel, in the monospace font. Long lines wrap.
fn code_block<'a, Message: 'a>(code: &'a str) -> Element<'a, Message> {
    container(
        text(code)
            .size(CODE_SIZE)
            .font(fonts::JetBrainsMono::REGULAR)
            .color(color::text_default())
            .wrapping(Wrapping::WordOrGlyph)
            .width(Fill),
    )
    .padding(12)
    .width(Fill)
    .style(panel_style)
    .into()
}

/// What stands in for a picture until it loads, or when it cannot: its alt text, or a picture glyph when it has none,
/// in a panel the size the HTML gave the picture when it gave both sides.
fn placeholder<'a, Message: 'a>(picture: &'a Picture) -> Element<'a, Message> {
    let content: Element<'a, Message> = if picture.alt.is_empty() {
        utility::icon(Icon::lucide().image(), 18.0, Some((color::text_placeholder(), color::text_placeholder())))
    } else {
        text(picture.alt.as_str())
            .size(12)
            .font(fonts::Inter::REGULAR)
            .color(color::text_muted())
            .into()
    };
    let (width, height) = match (picture.width, picture.height) {
        (Some(width), Some(height)) => (Length::Fixed(width), Length::Fixed(height)),
        _ if picture.alt.is_empty() => (Length::Fixed(PLACEHOLDER), Length::Fixed(PLACEHOLDER)),
        _ => (Length::Shrink, Length::Shrink),
    };
    container(content)
        .width(width)
        .height(height)
        .padding(10)
        .align_x(alignment::Horizontal::Center)
        .align_y(alignment::Vertical::Center)
        .style(panel_style)
        .into()
}

/// The panel behind placeholders and code.
fn panel_style(_: &Theme) -> container::Style {
    container::Style {
        background: Some(Background::Color(color::surface_bg3())),
        border: Border {
            radius: RADIUS.into(),
            ..Border::default()
        },
        ..container::Style::default()
    }
}

/// A heading's text size, by its level, in the ratios iced's markdown uses.
fn heading_size(level: u8) -> f32 {
    TEXT_SIZE
        * match level {
            1 => 2.0,
            2 => 1.75,
            3 => 1.5,
            4 => 1.25,
            _ => 1.0,
        }
}

/// The horizontal alignment of a block's alignment.
fn horizontal(align: Align) -> alignment::Horizontal {
    match align {
        Align::Left => alignment::Horizontal::Left,
        Align::Center => alignment::Horizontal::Center,
        Align::Right => alignment::Horizontal::Right,
    }
}

/// How many digits a number is written with.
fn digits(number: u32) -> u32 {
    number.checked_ilog10().unwrap_or_default() + 1
}
