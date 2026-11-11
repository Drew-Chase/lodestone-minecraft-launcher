//! HTML drawn as the launcher's own elements. CurseForge writes its project descriptions in HTML, which iced's markdown
//! viewer cannot show. [`parse`] reads a document once into a [`Document`]: styled text, rows of pictures, lists,
//! quotes, code, rules, tables and collapsible spoilers. [`view`] draws it.
//!
//! Reading runs in three passes: [`tokens`] splits the HTML into tags and text, [`tree`] builds them into elements
//! with the recovery HTML is written to expect, and [`blocks`] turns the elements into blocks. [`css`] reads the inline
//! styles and [`entities`] the character references. The HTML comes from the network, so no pass runs anything it
//! holds, and every pass is bounded: the tree's depth, the blocks' nesting, and the addresses the caller accepts.

mod blocks;
mod css;
mod entities;
mod tokens;
mod tree;
mod view;

use std::collections::HashSet;

use iced::Color;

pub use view::{Viewer, view};

/// A document read into the blocks the page draws.
#[derive(Debug, Clone, Default, PartialEq)]
pub struct Document {
    /// The blocks, in reading order.
    pub blocks: Vec<Block>,
    /// Every address the document links to, once each. Spans and pictures name theirs by its index here.
    pub links: Vec<String>,
}

impl Document {
    /// Whether the document has nothing to draw.
    pub fn is_empty(&self) -> bool {
        self.blocks.is_empty()
    }

    /// The addresses of the pictures a reader can see, each once, in the order they first appear. A picture inside a
    /// spoiler counts only while the spoiler is in `open`.
    pub fn pictures(&self, open: &HashSet<usize>) -> Vec<&str> {
        let mut found = Vec::new();
        collect_pictures(&self.blocks, open, &mut found);
        let mut seen = HashSet::new();
        found.retain(|url| seen.insert(*url));
        found
    }
}

/// Adds the pictures of `blocks` to `found`, in reading order, descending into the spoilers in `open`.
fn collect_pictures<'a>(blocks: &'a [Block], open: &HashSet<usize>, found: &mut Vec<&'a str>) {
    for block in blocks {
        match block {
            Block::Pictures { pictures, .. } => found.extend(pictures.iter().map(|picture| picture.url.as_str())),
            Block::List { items, .. } => items.iter().for_each(|item| collect_pictures(item, open, found)),
            Block::Quote(blocks) => collect_pictures(blocks, open, found),
            Block::Table(rows) => rows.iter().flatten().for_each(|cell| collect_pictures(cell, open, found)),
            Block::Spoiler { index, blocks, .. } if open.contains(index) => collect_pictures(blocks, open, found),
            Block::Text { .. } | Block::Code(_) | Block::Rule | Block::Spoiler { .. } => {}
        }
    }
}

/// Reads HTML into a document. `resolve` turns each link and picture address into a full web address, or rejects it
/// with `None`: a rejected picture is left out, and rejected link text is drawn without a link.
pub fn parse(html: &str, resolve: fn(&str) -> Option<String>) -> Document {
    let root = tree::build(tokens::tokenize(html));
    blocks::build(&root, resolve)
}

/// One block of a document.
#[derive(Debug, Clone, PartialEq)]
pub enum Block {
    /// A paragraph, or a heading of level 1 to 6.
    Text { spans: Vec<Span>, heading: Option<u8>, align: Align },
    /// Pictures side by side, wrapping onto more lines when they run out of room.
    Pictures { pictures: Vec<Picture>, align: Align },
    /// A list: numbered from `start` when it is ordered, bulleted when `start` is `None`. Each item is blocks.
    List { start: Option<u32>, items: Vec<Vec<Block>> },
    /// A quotation.
    Quote(Vec<Block>),
    /// Preformatted text, drawn as written.
    Code(String),
    /// A line across the page.
    Rule,
    /// A table: rows of cells, each cell blocks.
    Table(Vec<Vec<Vec<Block>>>),
    /// Blocks hidden until the reader opens them. `index` numbers the document's spoilers in reading order, and
    /// `title` is what the toggle says, when the document named it.
    Spoiler {
        index: usize,
        title: Option<String>,
        blocks: Vec<Block>,
    },
}

/// A run of text in one style, with the link it makes.
#[derive(Debug, Clone, PartialEq)]
pub struct Span {
    pub text: String,
    pub style: Style,
    /// The link's index in [`Document::links`].
    pub link: Option<usize>,
}

/// How a run of text is drawn.
#[derive(Debug, Clone, Copy, Default, PartialEq)]
pub struct Style {
    pub bold: bool,
    pub italic: bool,
    pub underline: bool,
    pub strike: bool,
    /// Code, drawn in the monospace font.
    pub code: bool,
    /// The author's colour, drawn only where it is readable.
    pub color: Option<Color>,
    /// The author's size, in the page's pixels.
    pub size: Option<f32>,
}

/// A picture, or a video drawn as its thumbnail.
#[derive(Debug, Clone, PartialEq)]
pub struct Picture {
    /// The picture's web address.
    pub url: String,
    /// What the picture shows, for while it loads or when it cannot be had.
    pub alt: String,
    /// The link a press opens: its index in [`Document::links`].
    pub link: Option<usize>,
    /// The width the HTML gave it, in pixels.
    pub width: Option<f32>,
    /// The height the HTML gave it, in pixels.
    pub height: Option<f32>,
    /// Whether the picture is a video's thumbnail, drawn with a play mark.
    pub video: bool,
}

/// Where a block's lines sit.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub enum Align {
    #[default]
    Left,
    Center,
    Right,
}

#[cfg(test)]
#[path = "../../../../tests/app/components/html/mod.rs"]
mod tests;
