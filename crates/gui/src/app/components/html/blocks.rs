//! The element tree read into the blocks the page draws. Text flows into paragraphs and headings with the styles and
//! link around it. Pictures that sit together gather into a row, and text that shows something ends the row. Lists,
//! quotes, code, rules, tables and spoilers become blocks of their own. Every address goes through the caller's
//! `resolve`, which makes it a full web address or rejects it.

use std::collections::HashMap;

use super::css;
use super::tree::{Element, Node};
use super::{Align, Block, Document, Picture, Span, Style};

/// How deep blocks nest: lists in quotes in spoilers. A container deeper than this flows into its parent, so a page
/// cannot grow the drawn tree without limit.
pub const MAX_NESTING: usize = 8;

/// The width of a video's thumbnail when the embed names none, as YouTube's own embed code does.
pub const VIDEO_WIDTH: f32 = 560.0;

/// The most newlines that run together: one blank line.
const MAX_BREAKS: usize = 2;

/// Elements skipped with everything inside them: what a description cannot run or show.
const SKIPPED: &[&str] = &[
    "script", "style", "head", "title", "noscript", "template", "object", "embed", "audio", "video", "canvas", "svg", "math", "button", "input",
    "select", "option", "textarea", "map", "area", "source", "track", "param", "meta", "link", "base", "col", "colgroup",
];

/// Elements that are blocks: the text before them ends, and the text inside them starts on a line of its own.
const BLOCKS: &[&str] = &[
    "address",
    "article",
    "aside",
    "blockquote",
    "body",
    "caption",
    "center",
    "dd",
    "details",
    "dialog",
    "dir",
    "div",
    "dl",
    "dt",
    "fieldset",
    "figcaption",
    "figure",
    "footer",
    "form",
    "header",
    "hgroup",
    "html",
    "legend",
    "li",
    "main",
    "menu",
    "nav",
    "ol",
    "p",
    "section",
    "summary",
    "table",
    "tbody",
    "td",
    "tfoot",
    "th",
    "thead",
    "tr",
    "ul",
];

/// Reads the tree under `root` into a document.
pub fn build(root: &Element, resolve: fn(&str) -> Option<String>) -> Document {
    let mut builder = Builder {
        resolve,
        links: Vec::new(),
        known: HashMap::new(),
        spoilers: 0,
    };
    let mut flow = Flow::new(Align::Left);
    builder.children(root, &mut flow, Inline::default(), 0);
    Document {
        blocks: flow.finish(),
        links: builder.links,
    }
}

/// What the reading keeps across the whole document: the links made so far and the spoilers numbered so far.
struct Builder {
    resolve: fn(&str) -> Option<String>,
    links: Vec<String>,
    /// Each link's index in `links`, so a link used twice is kept once.
    known: HashMap<String, usize>,
    spoilers: usize,
}

/// The style and link the elements around some text give it.
#[derive(Debug, Clone, Copy, Default)]
struct Inline {
    style: Style,
    link: Option<usize>,
}

impl Builder {
    /// Reads an element's children into `flow`.
    fn children(&mut self, element: &Element, flow: &mut Flow, inline: Inline, nesting: usize) {
        for child in &element.children {
            self.node(child, flow, inline, nesting);
        }
    }

    /// Reads one node into `flow`.
    fn node(&mut self, node: &Node, flow: &mut Flow, inline: Inline, nesting: usize) {
        match node {
            Node::Text(text) => flow.text(text, inline),
            Node::Element(element) => self.element(element, flow, inline, nesting),
        }
    }

    /// Reads one element into `flow`. `nesting` counts the containers around it that draw as blocks of their own.
    fn element(&mut self, element: &Element, flow: &mut Flow, inline: Inline, nesting: usize) {
        let name = element.name.as_str();
        if SKIPPED.contains(&name) {
            return;
        }
        let nests = nesting < MAX_NESTING;
        match name {
            "br" => flow.line_break(),
            "img" => {
                if let Some(picture) = self.picture(element, inline) {
                    flow.picture(picture);
                }
            }
            "iframe" => self.embed(element, flow, inline),
            "hr" => {
                flow.flush();
                flow.blocks.push(Block::Rule);
            }
            "pre" => {
                flow.flush();
                let code = code_text(element);
                if !code.trim().is_empty() {
                    flow.blocks.push(Block::Code(code));
                }
            }
            "h1" | "h2" | "h3" | "h4" | "h5" | "h6" => {
                let level = name.as_bytes()[1] - b'0';
                self.heading(element, level, flow, inline, nesting);
            }
            "ul" | "ol" | "menu" | "dir" if nests => self.list(element, flow, inline, nesting),
            "blockquote" if nests => {
                flow.flush();
                let mut inner = Flow::new(alignment(element).unwrap_or_default());
                let inline = self.inline(element, inline);
                self.children(element, &mut inner, inline, nesting + 1);
                let blocks = inner.finish();
                if !blocks.is_empty() {
                    flow.blocks.push(Block::Quote(blocks));
                }
            }
            "table" if nests => self.table(element, flow, inline, nesting),
            "details" if nests => self.details(element, flow, inline, nesting),
            _ if nests && element.has_class("spoiler") => self.spoiler(element, None, None, flow, inline, nesting),
            _ if BLOCKS.contains(&name) || element.has_class("spoiler") => self.block(element, flow, inline, nesting),
            _ => {
                let inline = self.inline(element, inline);
                self.children(element, flow, inline, nesting);
            }
        }
    }

    /// A block that draws nothing of its own: its content starts on a line of its own, in its alignment.
    fn block(&mut self, element: &Element, flow: &mut Flow, inline: Inline, nesting: usize) {
        flow.flush();
        let outer = flow.align;
        if let Some(align) = alignment(element) {
            flow.align = align;
        }
        let inline = self.inline(element, inline);
        self.children(element, flow, inline, nesting);
        flow.flush();
        flow.align = outer;
    }

    /// A heading. A picture inside it splits off as a row of its own.
    fn heading(&mut self, element: &Element, level: u8, flow: &mut Flow, inline: Inline, nesting: usize) {
        flow.flush();
        let (outer_align, outer_heading) = (flow.align, flow.heading);
        if let Some(align) = alignment(element) {
            flow.align = align;
        }
        flow.heading = Some(level);
        let inline = self.inline(element, inline);
        self.children(element, flow, inline, nesting);
        flow.flush();
        flow.align = outer_align;
        flow.heading = outer_heading;
    }

    /// A list. Its items are the `li` children. Anything else inside it joins the item before it, or makes an item of
    /// its own when it comes first.
    fn list(&mut self, element: &Element, flow: &mut Flow, inline: Inline, nesting: usize) {
        flow.flush();
        let start = (element.name == "ol").then(|| element.attribute("start").and_then(|start| start.trim().parse().ok()).unwrap_or(1));
        let inline = self.inline(element, inline);
        let mut items: Vec<Vec<Block>> = Vec::new();
        let mut stray = Flow::new(Align::Left);
        for child in &element.children {
            match child {
                Node::Element(item) if item.name == "li" => {
                    join_stray(&mut items, &mut stray);
                    let mut content = Flow::new(alignment(item).unwrap_or_default());
                    let inline = self.inline(item, inline);
                    self.children(item, &mut content, inline, nesting + 1);
                    let blocks = content.finish();
                    if !blocks.is_empty() {
                        items.push(blocks);
                    }
                }
                other => self.node(other, &mut stray, inline, nesting + 1),
            }
        }
        join_stray(&mut items, &mut stray);
        if !items.is_empty() {
            flow.blocks.push(Block::List { start, items });
        }
    }

    /// A table: its rows, from the table itself or from its `thead`, `tbody` and `tfoot`. A caption, and anything
    /// else written outside the rows, is drawn above the table.
    fn table(&mut self, element: &Element, flow: &mut Flow, inline: Inline, nesting: usize) {
        flow.flush();
        let inline = self.inline(element, inline);
        let mut above = Flow::new(flow.align);
        let mut rows: Vec<Vec<Vec<Block>>> = Vec::new();
        self.table_part(element, &mut above, &mut rows, inline, nesting);
        flow.blocks.extend(above.finish());
        if !rows.is_empty() {
            flow.blocks.push(Block::Table(rows));
        }
    }

    /// The rows of a table or of a section of one. What is not a row goes into `above`.
    fn table_part(&mut self, part: &Element, above: &mut Flow, rows: &mut Vec<Vec<Vec<Block>>>, inline: Inline, nesting: usize) {
        for child in &part.children {
            match child {
                Node::Element(section) if matches!(section.name.as_str(), "thead" | "tbody" | "tfoot") => {
                    let inline = self.inline(section, inline);
                    self.table_part(section, above, rows, inline, nesting);
                }
                Node::Element(row) if row.name == "tr" => {
                    let inline = self.inline(row, inline);
                    let cells: Vec<Vec<Block>> = row
                        .children
                        .iter()
                        .filter_map(|cell| match cell {
                            Node::Element(cell) if matches!(cell.name.as_str(), "td" | "th") => Some(cell),
                            _ => None,
                        })
                        .map(|cell| {
                            let mut inline = self.inline(cell, inline);
                            inline.style.bold |= cell.name == "th";
                            let mut content = Flow::new(alignment(cell).or_else(|| alignment(row)).unwrap_or_default());
                            self.children(cell, &mut content, inline, nesting + 1);
                            content.finish()
                        })
                        .collect();
                    if !cells.is_empty() {
                        rows.push(cells);
                    }
                }
                Node::Element(caption) if caption.name == "caption" => self.block(caption, above, inline, nesting),
                other => self.node(other, above, inline, nesting),
            }
        }
    }

    /// A `details` element: a spoiler titled by its first `summary`.
    fn details(&mut self, element: &Element, flow: &mut Flow, inline: Inline, nesting: usize) {
        let summary = element
            .children
            .iter()
            .position(|child| matches!(child, Node::Element(summary) if summary.name == "summary"));
        let title = summary
            .and_then(|at| match &element.children[at] {
                Node::Element(summary) => Some(collapse(&summary.text_content()).trim().to_owned()),
                Node::Text(_) => None,
            })
            .filter(|title| !title.is_empty());
        self.spoiler(element, title, summary, flow, inline, nesting);
    }

    /// A spoiler: the element's content, hidden until the reader opens it. `skip` is a child left out, the summary
    /// that already titles it.
    fn spoiler(&mut self, element: &Element, title: Option<String>, skip: Option<usize>, flow: &mut Flow, inline: Inline, nesting: usize) {
        flow.flush();
        let index = self.spoilers;
        self.spoilers += 1;
        let mut inner = Flow::new(alignment(element).unwrap_or(flow.align));
        let inline = self.inline(element, inline);
        for (at, child) in element.children.iter().enumerate() {
            if Some(at) != skip {
                self.node(child, &mut inner, inline, nesting + 1);
            }
        }
        let blocks = inner.finish();
        if !blocks.is_empty() {
            flow.blocks.push(Block::Spoiler { index, title, blocks });
        }
    }

    /// An `iframe`. A YouTube video becomes its thumbnail, linked to the video. Any other embed becomes a line with its
    /// address as a link.
    fn embed(&mut self, element: &Element, flow: &mut Flow, inline: Inline) {
        let Some(source) = element.attribute("src").and_then(|source| (self.resolve)(source.trim())) else {
            return;
        };
        match youtube_id(&source) {
            Some(id) => {
                let link = self.link(format!("https://www.youtube.com/watch?v={id}"));
                flow.picture(Picture {
                    url: format!("https://i.ytimg.com/vi/{id}/hqdefault.jpg"),
                    alt: String::new(),
                    link: Some(link),
                    width: dimension(element, "width").or(Some(VIDEO_WIDTH)),
                    height: None,
                    video: true,
                });
            }
            None => {
                flow.flush();
                let link = self.link(source.clone());
                flow.text(&source, Inline { link: Some(link), ..inline });
                flow.flush();
            }
        }
    }

    /// An `img` as a picture, with the link around it. `None` when its address is rejected.
    fn picture(&mut self, element: &Element, inline: Inline) -> Option<Picture> {
        let url = element.attribute("src").and_then(|source| (self.resolve)(source.trim()))?;
        Some(Picture {
            url,
            alt: collapse(element.attribute("alt").unwrap_or_default()).trim().to_owned(),
            link: inline.link,
            width: dimension(element, "width"),
            height: dimension(element, "height"),
            video: false,
        })
    }

    /// The style and link an element gives the text inside it: from what it is, from its presentational attributes,
    /// and from its `style`.
    fn inline(&mut self, element: &Element, mut inline: Inline) -> Inline {
        match element.name.as_str() {
            "b" | "strong" => inline.style.bold = true,
            "i" | "em" | "cite" | "var" | "dfn" => inline.style.italic = true,
            "u" | "ins" => inline.style.underline = true,
            "s" | "strike" | "del" => inline.style.strike = true,
            "code" | "kbd" | "samp" | "tt" => inline.style.code = true,
            "font" => {
                if let Some(color) = element.attribute("color").and_then(css::color) {
                    inline.style.color = Some(color);
                }
                if let Some(size) = element.attribute("size").and_then(css::font_size_attribute) {
                    inline.style.size = Some(size);
                }
            }
            "a" => {
                if let Some(href) = element.attribute("href") {
                    inline.link = (self.resolve)(href.trim()).map(|url| self.link(url));
                }
            }
            _ => {}
        }
        if let Some(style) = element.attribute("style") {
            css::apply(style, &mut inline.style);
        }
        inline
    }

    /// The index of a link, kept once however often it is used.
    fn link(&mut self, url: String) -> usize {
        if let Some(&index) = self.known.get(&url) {
            return index;
        }
        let index = self.links.len();
        self.links.push(url.clone());
        self.known.insert(url, index);
        index
    }
}

/// The blocks being read in one container: the finished ones, and the line or row of pictures still open.
struct Flow {
    blocks: Vec<Block>,
    /// The line of text being read. It is empty while a row of pictures is open.
    spans: Vec<Span>,
    /// The row of pictures being read. It is empty while a line of text is open.
    pictures: Vec<Picture>,
    /// The alignment of the blocks being read.
    align: Align,
    /// The level of the heading being read, when the text is a heading.
    heading: Option<u8>,
}

impl Flow {
    fn new(align: Align) -> Flow {
        Flow {
            blocks: Vec::new(),
            spans: Vec::new(),
            pictures: Vec::new(),
            align,
            heading: None,
        }
    }

    /// Adds text. Runs of whitespace collapse to one space, and a line drops the spaces it would start with. Text that
    /// shows something ends a row of pictures. Whitespace between pictures is dropped.
    fn text(&mut self, raw: &str, inline: Inline) {
        let collapsed = collapse(raw);
        let text = if self.at_line_start() {
            collapsed.trim_start_matches(' ')
        } else {
            collapsed.as_str()
        };
        if text.is_empty() {
            return;
        }
        // A row of pictures is open only while no text is, so the line starts here and the text is already trimmed.
        if !self.pictures.is_empty() {
            if text.chars().all(char::is_whitespace) {
                return;
            }
            self.flush_pictures();
        }
        match self.spans.last_mut() {
            Some(last) if last.style == inline.style && last.link == inline.link => last.text.push_str(text),
            _ => self.spans.push(Span {
                text: text.to_owned(),
                style: inline.style,
                link: inline.link,
            }),
        }
    }

    /// Ends the line. A break ends a row of pictures too, and adds nothing at the start of a block. At most one blank
    /// line is kept.
    fn line_break(&mut self) {
        if !self.pictures.is_empty() {
            self.flush_pictures();
            return;
        }
        let Some(last) = self.spans.last_mut() else {
            return;
        };
        let kept = last.text.trim_end_matches(' ').len();
        last.text.truncate(kept);
        let breaks = last.text.chars().rev().take_while(|character| *character == '\n').count();
        if breaks < MAX_BREAKS {
            last.text.push('\n');
        }
    }

    /// Adds a picture to the open row, ending the line of text before it.
    fn picture(&mut self, picture: Picture) {
        self.flush_text();
        self.pictures.push(picture);
    }

    /// Ends the open line or row.
    fn flush(&mut self) {
        self.flush_text();
        self.flush_pictures();
    }

    /// Ends the line of text. A line that shows nothing (only spaces, as `<p>&nbsp;</p>` writes) is dropped, and the
    /// whitespace it ends with is trimmed.
    fn flush_text(&mut self) {
        let mut spans = std::mem::take(&mut self.spans);
        while let Some(last) = spans.last_mut() {
            let kept = last.text.trim_end().len();
            last.text.truncate(kept);
            if !last.text.is_empty() {
                break;
            }
            spans.pop();
        }
        if spans.iter().all(|span| span.text.chars().all(char::is_whitespace)) {
            return;
        }
        self.blocks.push(Block::Text {
            spans,
            heading: self.heading,
            align: self.align,
        });
    }

    /// Ends the row of pictures.
    fn flush_pictures(&mut self) {
        if !self.pictures.is_empty() {
            self.blocks.push(Block::Pictures {
                pictures: std::mem::take(&mut self.pictures),
                align: self.align,
            });
        }
    }

    /// Whether text added now starts a line: nothing is on the line yet, or it ends in a space or a break.
    fn at_line_start(&self) -> bool {
        self.spans
            .last()
            .is_none_or(|last| last.text.is_empty() || last.text.ends_with([' ', '\n']))
    }

    /// The finished blocks.
    fn finish(mut self) -> Vec<Block> {
        self.flush();
        self.blocks
    }
}

/// Puts the blocks read outside any item into the item before them, or into an item of their own when no item came
/// before.
fn join_stray(items: &mut Vec<Vec<Block>>, stray: &mut Flow) {
    let blocks = std::mem::replace(stray, Flow::new(Align::Left)).finish();
    if blocks.is_empty() {
        return;
    }
    match items.last_mut() {
        Some(last) => last.extend(blocks),
        None => items.push(blocks),
    }
}

/// The alignment an element sets: from `text-align` in its style, an `align` attribute, or being `center`.
fn alignment(element: &Element) -> Option<Align> {
    if element.name == "center" {
        return Some(Align::Center);
    }
    element
        .attribute("style")
        .and_then(|style| css::declaration(style, "text-align"))
        .and_then(css::align)
        .or_else(|| element.attribute("align").and_then(css::align))
}

/// A picture's width or height in pixels, from its style or else its attribute.
fn dimension(element: &Element, name: &str) -> Option<f32> {
    element
        .attribute("style")
        .and_then(|style| css::declaration(style, name))
        .and_then(css::pixels)
        .or_else(|| element.attribute(name).and_then(css::pixels))
}

/// The text of a `pre` as written, without the line break HTML drops after the opening tag or the whitespace it ends
/// with.
fn code_text(element: &Element) -> String {
    let text = element.text_content();
    let text = text.strip_prefix("\r\n").or_else(|| text.strip_prefix('\n')).unwrap_or(&text);
    text.trim_end().to_owned()
}

/// The text with each run of HTML whitespace (space, tab, line feed, form feed, carriage return) made one space. A
/// no-break space is not HTML whitespace, so it stays.
fn collapse(text: &str) -> String {
    let mut out = String::with_capacity(text.len());
    let mut space = false;
    for character in text.chars() {
        if matches!(character, ' ' | '\t' | '\n' | '\r' | '\u{C}') {
            space = true;
        } else {
            if space {
                out.push(' ');
                space = false;
            }
            out.push(character);
        }
    }
    if space {
        out.push(' ');
    }
    out
}

/// The id of a YouTube video an embed address shows: `youtube.com/embed/ID`, `youtube-nocookie.com/embed/ID` or
/// `youtu.be/ID`.
fn youtube_id(url: &str) -> Option<&str> {
    let rest = url.strip_prefix("https://").or_else(|| url.strip_prefix("http://"))?;
    let (host, path) = rest.split_once('/')?;
    let host = host.to_ascii_lowercase();
    let host = host.strip_prefix("www.").unwrap_or(&host);
    let id = match host {
        "youtube.com" | "youtube-nocookie.com" | "m.youtube.com" => path.strip_prefix("embed/")?,
        "youtu.be" => path,
        _ => return None,
    };
    let id = id.split(['?', '&', '#', '/']).next()?;
    let valid = id.len() >= 6 && id.bytes().all(|byte| byte.is_ascii_alphanumeric() || byte == b'-' || byte == b'_');
    valid.then_some(id)
}

#[cfg(test)]
#[path = "../../../../tests/app/components/html/blocks.rs"]
mod tests;
