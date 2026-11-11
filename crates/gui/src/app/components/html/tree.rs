//! The tokens built into a tree of elements and text, with the recovery HTML is written to expect. Void elements take
//! no children. A block closes an open paragraph, and a list item, table row or cell closes the one before it. An end
//! tag with no element open to close is ignored. The tree is at most [`MAX_DEPTH`] elements deep, so a hostile page
//! cannot make the passes over it recurse without limit.

use super::tokens::Token;

/// The deepest the tree grows. A start tag below this depth is dropped, and its content joins the element above it.
pub const MAX_DEPTH: usize = 64;

/// Elements that never have children. An `iframe`'s fallback content is never drawn, so it joins them.
const VOID: &[&str] = &[
    "area", "base", "br", "col", "embed", "hr", "img", "input", "link", "meta", "param", "source", "track", "wbr", "iframe",
];

/// Elements whose start closes an open paragraph.
const CLOSES_PARAGRAPH: &[&str] = &[
    "address",
    "article",
    "aside",
    "blockquote",
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
    "h1",
    "h2",
    "h3",
    "h4",
    "h5",
    "h6",
    "header",
    "hgroup",
    "hr",
    "li",
    "main",
    "menu",
    "nav",
    "ol",
    "p",
    "pre",
    "section",
    "summary",
    "table",
    "ul",
];

/// Elements a search for an open paragraph does not look past.
const PARAGRAPH_SCOPE: &[&str] = &[
    "button", "caption", "html", "table", "td", "th", "template", "object", "marquee", "applet",
];

/// Elements a search for an open list item does not look past.
const ITEM_SCOPE: &[&str] = &[
    "ul",
    "ol",
    "menu",
    "dir",
    "dl",
    "table",
    "td",
    "th",
    "caption",
    "template",
    "button",
    "blockquote",
    "details",
];

/// Elements a search for an open link does not look past.
const LINK_SCOPE: &[&str] = &["div", "p", "li", "td", "th", "table", "ul", "ol", "blockquote", "section", "details"];

/// Elements an end tag does not close past, unless it names one of them.
const END_SCOPE: &[&str] = &["table", "td", "th", "caption"];

/// One node of the tree.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Node {
    /// An element and what it holds.
    Element(Element),
    /// Text, with its references decoded.
    Text(String),
}

/// An element: its lowercase name, its attributes in the order written, and its children. The root has an empty name.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct Element {
    pub name: String,
    pub attributes: Vec<(String, String)>,
    pub children: Vec<Node>,
}

impl Element {
    /// An element with no attributes or children.
    pub fn named(name: &str) -> Element {
        Element {
            name: name.to_owned(),
            ..Element::default()
        }
    }

    /// The value of an attribute, by its lowercase name.
    pub fn attribute(&self, name: &str) -> Option<&str> {
        self.attributes.iter().find(|(key, _)| key == name).map(|(_, value)| value.as_str())
    }

    /// Whether the element's `class` names `class`, without regard to case.
    pub fn has_class(&self, class: &str) -> bool {
        self.attribute("class")
            .is_some_and(|value| value.split_ascii_whitespace().any(|token| token.eq_ignore_ascii_case(class)))
    }

    /// The element's text as written, its line breaks included: what a `pre` shows.
    pub fn text_content(&self) -> String {
        let mut out = String::new();
        self.write_text(&mut out);
        out
    }

    fn write_text(&self, out: &mut String) {
        for child in &self.children {
            match child {
                Node::Text(text) => out.push_str(text),
                Node::Element(element) if element.name == "br" => out.push('\n'),
                Node::Element(element) => element.write_text(out),
            }
        }
    }
}

/// Builds the tree of a document's tokens. The root holds the document.
pub fn build(tokens: Vec<Token<'_>>) -> Element {
    let mut builder = Builder {
        stack: vec![Element::default()],
        dropped: 0,
    };
    for token in tokens {
        match token {
            Token::Start { name, attributes, .. } => builder.start(name, attributes),
            Token::End(name) => builder.end(&name),
            Token::Text(text) => builder.text(&text),
        }
    }
    builder.close_to(1);
    builder.stack.pop().unwrap_or_default()
}

/// The elements still open, the root first, and how many start tags were dropped below the depth limit.
struct Builder {
    stack: Vec<Element>,
    /// Start tags dropped at the depth limit. Each takes the next end tag with it.
    dropped: usize,
}

impl Builder {
    /// Opens an element, closing first the ones its start ends.
    fn start(&mut self, name: String, attributes: Vec<(String, String)>) {
        if CLOSES_PARAGRAPH.contains(&name.as_str())
            && let Some(at) = self.open("p", PARAGRAPH_SCOPE)
        {
            self.close_to(at);
        }
        match name.as_str() {
            "li" => self.close_open(&["li"], ITEM_SCOPE),
            "dt" | "dd" => self.close_open(&["dt", "dd"], ITEM_SCOPE),
            "tr" => self.close_open(&["tr"], &["table", "thead", "tbody", "tfoot"]),
            "td" | "th" => self.close_open(&["td", "th"], &["tr", "table"]),
            "thead" | "tbody" | "tfoot" => self.close_open(&["thead", "tbody", "tfoot"], &["table"]),
            "h1" | "h2" | "h3" | "h4" | "h5" | "h6" => {
                if self.stack.len() > 1 && is_heading(&self.current().name) {
                    self.pop();
                }
            }
            "a" => self.close_open(&["a"], LINK_SCOPE),
            _ => {}
        }
        let element = Element {
            name,
            attributes,
            children: Vec::new(),
        };
        if VOID.contains(&element.name.as_str()) {
            self.current().children.push(Node::Element(element));
        } else if self.stack.len() > MAX_DEPTH {
            self.dropped += 1;
        } else {
            self.stack.push(element);
        }
    }

    /// Closes the element an end tag names, and the ones open inside it. An end tag with nothing to close is ignored,
    /// except `</br>`, which browsers read as a line break.
    fn end(&mut self, name: &str) {
        if self.dropped > 0 {
            self.dropped -= 1;
            return;
        }
        if name == "br" {
            self.current().children.push(Node::Element(Element::named("br")));
            return;
        }
        let scope = if END_SCOPE.contains(&name) { &[][..] } else { END_SCOPE };
        if let Some(at) = self.open(name, scope) {
            self.close_to(at);
        }
    }

    /// Adds text to the open element, joined to text just before it.
    fn text(&mut self, text: &str) {
        let current = self.current();
        if let Some(Node::Text(last)) = current.children.last_mut() {
            last.push_str(text);
        } else {
            current.children.push(Node::Text(text.to_owned()));
        }
    }

    /// The element the next node joins.
    fn current(&mut self) -> &mut Element {
        let last = self.stack.len() - 1;
        &mut self.stack[last]
    }

    /// Where the nearest open element named `name` sits, when no element in `scope` is open inside it.
    fn open(&self, name: &str, scope: &[&str]) -> Option<usize> {
        for (at, element) in self.stack.iter().enumerate().skip(1).rev() {
            if element.name == name {
                return Some(at);
            }
            if scope.contains(&element.name.as_str()) {
                return None;
            }
        }
        None
    }

    /// Closes the nearest open element with one of `names`, unless an element in `scope` is open inside it.
    fn close_open(&mut self, names: &[&str], scope: &[&str]) {
        let found = self.stack.iter().enumerate().skip(1).rev().find_map(|(at, element)| {
            if names.contains(&element.name.as_str()) {
                Some(Some(at))
            } else if scope.contains(&element.name.as_str()) {
                Some(None)
            } else {
                None
            }
        });
        if let Some(Some(at)) = found {
            self.close_to(at);
        }
    }

    /// Closes the element at `at` and every element open inside it. The root is never closed.
    fn close_to(&mut self, at: usize) {
        while self.stack.len() > at.max(1) {
            self.pop();
        }
    }

    /// Closes the innermost open element into its parent.
    fn pop(&mut self) {
        if self.stack.len() > 1
            && let Some(element) = self.stack.pop()
        {
            self.current().children.push(Node::Element(element));
        }
    }
}

/// Whether an element is a heading.
fn is_heading(name: &str) -> bool {
    matches!(name, "h1" | "h2" | "h3" | "h4" | "h5" | "h6")
}

#[cfg(test)]
#[path = "../../../../tests/app/components/html/tree.rs"]
mod tests;
