//! A patch note's HTML as markdown, for the reader. The markdown parser keeps headings, lists, links, emphasis and code, so
//! those tags become markdown; table cells become a line of text joined by bars; pictures become markdown images. Any
//! other tag goes, and its text stays. Text has its entities decoded, and the characters markdown would read as syntax
//! are escaped, except inside code, which is kept as written.

use super::feed::absolute;

/// The markdown of a patch note's HTML body.
pub fn to_markdown(html: &str) -> String {
    let mut converter = Converter::default();
    let mut rest = html;
    while let Some(start) = rest.find('<') {
        converter.text(&rest[..start]);
        let tail = &rest[start..];
        if tail.starts_with("<!--") {
            rest = tail.find("-->").map_or("", |end| &tail[end + 3..]);
            continue;
        }
        let opens_tag = tail
            .as_bytes()
            .get(1)
            .is_some_and(|byte| byte.is_ascii_alphabetic() || *byte == b'/' || *byte == b'!');
        if !opens_tag {
            converter.text("<");
            rest = &tail[1..];
            continue;
        }
        let Some(end) = tail.find('>') else {
            converter.text(tail);
            rest = "";
            break;
        };
        converter.tag(&tail[1..end]);
        rest = &tail[end + 1..];
    }
    converter.text(rest);
    converter.finish()
}

/// The list a `<ul>` or `<ol>` opened, with the number its last item got.
#[derive(Clone, Copy)]
enum List {
    Bullet,
    Numbered(u32),
}

/// The markdown written so far, and the tags still open around the text.
#[derive(Default)]
struct Converter {
    out: String,
    /// The lists open around the text, outermost first.
    lists: Vec<List>,
    /// The hrefs of the anchors open around the text, innermost last.
    links: Vec<String>,
    /// How deep the text is inside `<pre>`, where whitespace and symbols are kept as they are.
    pre: usize,
    /// How deep the text is inside `<code>`, where symbols are kept as they are.
    code: usize,
}

impl Converter {
    /// Writes text. Its whitespace collapses to single spaces, except inside code, and it is escaped unless it is code.
    fn text(&mut self, raw: &str) {
        if raw.is_empty() {
            return;
        }
        let decoded = decode_entities(raw);
        if self.pre > 0 {
            self.out.push_str(&decoded);
            return;
        }
        let mut collapsed = collapse(&decoded);
        if self.out.is_empty() || self.out.ends_with([' ', '\n']) {
            collapsed = collapsed.trim_start().to_owned();
        }
        if self.code > 0 {
            self.out.push_str(&collapsed);
        } else {
            self.out.push_str(&escape(&collapsed));
        }
    }

    /// Handles one tag, given without its angle brackets.
    fn tag(&mut self, raw: &str) {
        let closing = raw.starts_with('/');
        let body = raw.trim_start_matches('/').trim_end_matches('/');
        let name = body
            .split(|character: char| character.is_whitespace() || character == '/')
            .next()
            .unwrap_or_default()
            .to_ascii_lowercase();
        match (name.as_str(), closing) {
            ("p" | "div" | "section" | "table" | "thead" | "tbody" | "figure", _) => self.block(),
            ("h1" | "h2" | "h3" | "h4" | "h5" | "h6", false) => {
                self.block();
                let level = usize::from(name.as_bytes()[1] - b'0');
                self.out.push_str(&"#".repeat(level));
                self.out.push(' ');
            }
            ("h1" | "h2" | "h3" | "h4" | "h5" | "h6", true) => self.block(),
            ("br", _) => self.out.push_str("  \n"),
            ("ul", false) => self.open_list(List::Bullet),
            ("ol", false) => self.open_list(List::Numbered(0)),
            ("ul" | "ol", true) => {
                self.lists.pop();
                self.block();
            }
            ("li", false) => self.item(),
            ("strong" | "b", _) => self.out.push_str("**"),
            ("em" | "i", _) => self.out.push('*'),
            ("code", false) => {
                if self.pre == 0 {
                    self.out.push('`');
                }
                self.code += 1;
            }
            ("code", true) => {
                self.code = self.code.saturating_sub(1);
                if self.pre == 0 {
                    self.out.push('`');
                }
            }
            ("pre", false) => {
                self.block();
                self.out.push_str("```\n");
                self.pre += 1;
            }
            ("pre", true) => {
                self.pre = self.pre.saturating_sub(1);
                if self.pre == 0 {
                    self.break_line();
                    self.out.push_str("```\n");
                }
            }
            ("a", false) => {
                self.links
                    .push(attribute(body, "href").map(|href| decode_entities(&href)).unwrap_or_default());
                self.out.push('[');
            }
            ("a", true) => {
                let href = self.links.pop().unwrap_or_default();
                self.out.push(']');
                if !href.is_empty() {
                    self.out.push_str(&format!("({href})"));
                }
            }
            ("img", _) => {
                if let Some(source) = attribute(body, "src") {
                    let alt = attribute(body, "alt").unwrap_or_default().replace(['[', ']'], "");
                    self.out.push_str(&format!("![{alt}]({})", absolute(&decode_entities(&source))));
                }
            }
            ("tr", _) => self.break_line(),
            ("td" | "th", true) => self.out.push_str(" | "),
            _ => {}
        }
    }

    /// Starts a list: its first item follows on a line of its own.
    fn open_list(&mut self, list: List) {
        self.break_line();
        self.lists.push(list);
    }

    /// Starts a list item on a line of its own, indented under the lists around it.
    fn item(&mut self) {
        self.break_line();
        let indent = "  ".repeat(self.lists.len().saturating_sub(1));
        let marker = match self.lists.last_mut() {
            Some(List::Numbered(count)) => {
                *count += 1;
                format!("{count}. ")
            }
            _ => "- ".to_owned(),
        };
        self.out.push_str(&indent);
        self.out.push_str(&marker);
    }

    /// Starts a new paragraph: a blank line before it, unless the text is empty so far.
    fn block(&mut self) {
        if self.out.is_empty() || self.out.ends_with("\n\n") {
            return;
        }
        self.break_line();
        self.out.push('\n');
    }

    /// Ends the line the text is on, unless it is ended already or nothing is written yet. Spaces left at the end of
    /// the line by the text's whitespace go, so no line ends in a bare space.
    fn break_line(&mut self) {
        if self.out.is_empty() || self.out.ends_with('\n') {
            return;
        }
        let kept = self.out.trim_end_matches(' ').len();
        self.out.truncate(kept);
        self.out.push('\n');
    }

    /// The markdown, with no run of more than one blank line and no blank lines at either end.
    fn finish(self) -> String {
        let mut collapsed = String::with_capacity(self.out.len());
        let mut newlines = 0;
        for character in self.out.chars() {
            if character == '\n' {
                newlines += 1;
                if newlines > 2 {
                    continue;
                }
            } else {
                newlines = 0;
            }
            collapsed.push(character);
        }
        collapsed.trim().to_owned()
    }
}

/// Replaces each run of whitespace with a single space.
fn collapse(text: &str) -> String {
    let mut out = String::with_capacity(text.len());
    let mut pending_space = false;
    for character in text.chars() {
        if character.is_whitespace() {
            pending_space = true;
        } else {
            if pending_space {
                out.push(' ');
                pending_space = false;
            }
            out.push(character);
        }
    }
    if pending_space {
        out.push(' ');
    }
    out
}

/// Escapes the characters markdown reads as syntax, so text comes out as it was written.
fn escape(text: &str) -> String {
    let mut escaped = String::with_capacity(text.len());
    for character in text.chars() {
        if matches!(character, '\\' | '`' | '*' | '_' | '[' | ']' | '<' | '>') {
            escaped.push('\\');
        }
        escaped.push(character);
    }
    escaped
}

/// The text with its entities decoded: named ones the markup uses, and numeric ones in decimal or hexadecimal.
fn decode_entities(raw: &str) -> String {
    if !raw.contains('&') {
        return raw.to_owned();
    }
    let mut out = String::with_capacity(raw.len());
    let mut rest = raw;
    while let Some(at) = rest.find('&') {
        out.push_str(&rest[..at]);
        let tail = &rest[at..];
        match entity(tail) {
            Some((character, length)) => {
                out.push(character);
                rest = &tail[length..];
            }
            None => {
                out.push('&');
                rest = &tail[1..];
            }
        }
    }
    out.push_str(rest);
    out
}

/// The character an entity at the start of `text` stands for, and how many bytes the entity takes, when it is known.
fn entity(text: &str) -> Option<(char, usize)> {
    let end = text.find(';')?;
    if end > 10 {
        return None;
    }
    let name = &text[1..end];
    let character = if let Some(hex) = name.strip_prefix("#x").or_else(|| name.strip_prefix("#X")) {
        char::from_u32(u32::from_str_radix(hex, 16).ok()?)?
    } else if let Some(decimal) = name.strip_prefix('#') {
        char::from_u32(decimal.parse().ok()?)?
    } else {
        match name {
            "amp" => '&',
            "lt" => '<',
            "gt" => '>',
            "quot" => '"',
            "apos" => '\'',
            "nbsp" => ' ',
            _ => return None,
        }
    };
    Some((character, end + 1))
}

/// The value of an attribute in a tag, quoted with either quote mark or bare up to the next space. The name must start
/// a word, so `data-href` does not answer for `href`.
fn attribute(tag: &str, name: &str) -> Option<String> {
    let lower = tag.to_ascii_lowercase();
    let needle = format!("{name}=");
    let mut from = 0;
    while let Some(found) = lower[from..].find(&needle) {
        let at = from + found;
        let value_at = at + needle.len();
        let starts_word = at == 0 || lower.as_bytes()[at - 1].is_ascii_whitespace();
        if starts_word {
            let value = &tag[value_at..];
            return match value.chars().next() {
                Some(quote @ ('"' | '\'')) => value[1..].find(quote).map(|end| value[1..1 + end].to_owned()),
                Some(_) => value.split_whitespace().next().map(str::to_owned),
                None => None,
            };
        }
        from = value_at;
    }
    None
}

#[cfg(test)]
#[path = "../../../tests/app/news/markdown.rs"]
mod tests;
