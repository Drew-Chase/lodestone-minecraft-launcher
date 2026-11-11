//! An HTML document read into tokens: start tags, end tags and text. Comments, declarations and processing
//! instructions are skipped, and so is the content of the elements whose text is never drawn, such as `script` and
//! `style`. A `<` that opens no tag is text, and so is a tag that never closes, so nothing written is lost.

use std::borrow::Cow;

use super::entities;

/// One token of a document.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Token<'a> {
    /// A start tag: its lowercase name, its attributes in the order written (lowercase names, decoded values, the first
    /// of a repeated name kept), and whether it closed itself, as `<br/>` does.
    Start {
        name: String,
        attributes: Vec<(String, String)>,
        closed: bool,
    },
    /// An end tag, by its lowercase name.
    End(String),
    /// The text between tags, with its character references decoded.
    Text(Cow<'a, str>),
}

/// The elements whose content is never drawn. Their content is skipped up to their end tag.
const RAW: [&str; 6] = ["script", "style", "noscript", "template", "textarea", "title"];

/// Reads a document into its tokens.
pub fn tokenize(html: &str) -> Vec<Token<'_>> {
    let mut tokens = Vec::new();
    let mut rest = html;
    while let Some(at) = rest.find('<') {
        push_text(&mut tokens, &rest[..at]);
        let tail = &rest[at..];
        rest = match markup(tail) {
            Markup::Skip(length) => &tail[length..],
            Markup::Tag(token, length) => {
                let raw = match &token {
                    Token::Start { name, closed: false, .. } if RAW.contains(&name.as_str()) => Some(name.clone()),
                    _ => None,
                };
                tokens.push(token);
                let after = &tail[length..];
                match raw {
                    // The content is skipped, and the end tag is read as any other.
                    Some(name) => raw_end(after, &name).map_or("", |end| &after[end..]),
                    None => after,
                }
            }
            Markup::Text => {
                push_text(&mut tokens, "<");
                &tail[1..]
            }
            Markup::Unterminated => {
                push_text(&mut tokens, tail);
                ""
            }
        };
    }
    push_text(&mut tokens, rest);
    tokens
}

/// What a `<` opens.
enum Markup<'a> {
    /// A comment, declaration or processing instruction, taking this many bytes.
    Skip(usize),
    /// A tag, taking this many bytes.
    Tag(Token<'a>, usize),
    /// Nothing: the `<` is text.
    Text,
    /// A tag that never closes. The rest of the document is text.
    Unterminated,
}

/// Reads the markup at the start of `tail`, which starts with `<`.
fn markup(tail: &str) -> Markup<'static> {
    if let Some(comment) = tail.strip_prefix("<!--") {
        // `<!-->` and `<!--->` are empty comments that end at once.
        let length = if comment.starts_with('>') {
            5
        } else if comment.starts_with("->") {
            6
        } else {
            comment.find("-->").map_or(tail.len(), |end| 4 + end + 3)
        };
        return Markup::Skip(length);
    }
    if tail.starts_with("<!") || tail.starts_with("<?") {
        return Markup::Skip(tail.find('>').map_or(tail.len(), |end| end + 1));
    }
    let bytes = tail.as_bytes();
    if bytes.get(1) == Some(&b'/') && bytes.get(2).is_some_and(u8::is_ascii_alphabetic) {
        return match tail.find('>') {
            Some(end) => Markup::Tag(Token::End(name_of(&tail[2..end])), end + 1),
            None => Markup::Unterminated,
        };
    }
    if bytes.get(1).is_some_and(u8::is_ascii_alphabetic) {
        return match start_tag(tail) {
            Some((token, length)) => Markup::Tag(token, length),
            None => Markup::Unterminated,
        };
    }
    Markup::Text
}

/// A tag's lowercase name: the text up to the first space, `/` or `>`.
fn name_of(text: &str) -> String {
    text.split(|character: char| character.is_ascii_whitespace() || character == '/' || character == '>')
        .next()
        .unwrap_or_default()
        .to_ascii_lowercase()
}

/// A start tag at the start of `tail`, and the bytes it takes, or `None` when it never closes. A `>` inside a quoted
/// value does not close the tag.
fn start_tag(tail: &str) -> Option<(Token<'static>, usize)> {
    let bytes = tail.as_bytes();
    let mut at = 1;
    while at < bytes.len() && !bytes[at].is_ascii_whitespace() && bytes[at] != b'/' && bytes[at] != b'>' {
        at += 1;
    }
    let name = tail[1..at].to_ascii_lowercase();
    let mut attributes: Vec<(String, String)> = Vec::new();
    let mut closed = false;
    loop {
        while at < bytes.len() && bytes[at].is_ascii_whitespace() {
            at += 1;
        }
        match bytes.get(at)? {
            b'>' => return Some((Token::Start { name, attributes, closed }, at + 1)),
            b'/' => {
                at += 1;
                closed = bytes.get(at) == Some(&b'>');
                continue;
            }
            _ => {}
        }
        closed = false;
        let start = at;
        while at < bytes.len() && !bytes[at].is_ascii_whitespace() && !matches!(bytes[at], b'/' | b'>' | b'=') {
            at += 1;
        }
        // A stray `=` with no name before it is skipped as a name of its own.
        if at == start {
            at += 1;
        }
        let key = tail[start..at].to_ascii_lowercase();
        let mut probe = at;
        while probe < bytes.len() && bytes[probe].is_ascii_whitespace() {
            probe += 1;
        }
        let mut value = String::new();
        if bytes.get(probe) == Some(&b'=') {
            at = probe + 1;
            while at < bytes.len() && bytes[at].is_ascii_whitespace() {
                at += 1;
            }
            match bytes.get(at)? {
                quote @ (b'"' | b'\'') => match tail[at + 1..].find(*quote as char) {
                    Some(offset) => {
                        let end = at + 1 + offset;
                        value = entities::decode(&tail[at + 1..end]).into_owned();
                        at = end + 1;
                    }
                    // A value whose quote never closes runs to the next `>`, which closes the tag.
                    None => {
                        let end = at + 1 + tail[at + 1..].find('>')?;
                        value = entities::decode(&tail[at + 1..end]).into_owned();
                        at = end;
                    }
                },
                _ => {
                    let start = at;
                    while at < bytes.len() && !bytes[at].is_ascii_whitespace() && bytes[at] != b'>' {
                        at += 1;
                    }
                    value = entities::decode(&tail[start..at]).into_owned();
                }
            }
        }
        if !key.is_empty() && key != "=" && !attributes.iter().any(|(existing, _)| *existing == key) {
            attributes.push((key, value));
        }
    }
}

/// Where the end tag of the raw element `name` starts in `text`, matched without regard to case. `None` when it never
/// ends, and the rest of the document is its content.
fn raw_end(text: &str, name: &str) -> Option<usize> {
    text.match_indices("</").map(|(at, _)| at).find(|&at| {
        let after = &text[at + 2..];
        after.get(..name.len()).is_some_and(|candidate| candidate.eq_ignore_ascii_case(name))
            && after
                .as_bytes()
                .get(name.len())
                .is_none_or(|byte| byte.is_ascii_whitespace() || *byte == b'/' || *byte == b'>')
    })
}

/// Adds text with its references decoded. Empty text adds nothing.
fn push_text<'a>(tokens: &mut Vec<Token<'a>>, text: &'a str) {
    if !text.is_empty() {
        tokens.push(Token::Text(entities::decode(text)));
    }
}

#[cfg(test)]
#[path = "../../../../tests/app/components/html/tokens.rs"]
mod tests;
