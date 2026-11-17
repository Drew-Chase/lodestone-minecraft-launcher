//! Syntax colours for the editor: a lexer per language that reads one line at a time, and the highlighter that feeds
//! iced's text editor from it.
//!
//! A lexer is handed a line and what the line before left open (a block comment, a multi-line string), and gives back
//! the coloured spans of the line and what it leaves open in turn. The highlighter remembers what every line was
//! handed, so after an edit it starts again from the changed line rather than the top of the file.

mod crash;
mod function;
mod ini;
mod json;
mod log;
mod script;
mod toml;
mod xml;
mod yaml;

use std::ops::Range;

use iced::advanced::text::highlighter::Format;
use iced::{Font, Theme};

use super::language::Language;
use crate::app::theme::color;

/// What a span of text is, which decides its colour.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Token {
    Comment,
    /// The name of a setting: a JSON or YAML key, a TOML or properties name.
    Key,
    /// A quoted string, or a properties value.
    Text,
    Number,
    /// A word with a fixed meaning as a value: `true`, `null`, a selector such as `@a`.
    Constant,
    /// A word the language itself defines: a command, `function`, `import`.
    Keyword,
    /// Brackets, separators and operators.
    Punctuation,
    /// The heading of a group of settings, or of a part of a report.
    Section,
    /// A tag's name in XML, or a namespaced id such as `minecraft:stone`.
    Tag,
    /// An attribute's name in XML, or a type mark in an old Forge config.
    Attribute,
    /// A line that reports an error, or the level mark of one.
    Error,
    Warning,
    Info,
    Debug,
    /// Text that matters less than what is around it: a timestamp, a stack frame.
    Faint,
}

/// What a line leaves open for the next one.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum Carry {
    /// Nothing: the next line starts fresh.
    #[default]
    None,
    /// A block comment that has not ended.
    Comment,
    /// A multi-line string that has not ended, and the quote character that ends it.
    Text(u8),
    /// A YAML block scalar. The lines indented deeper than this many columns belong to it.
    Block(usize),
    /// An XML tag whose `>` has not come yet.
    Tag,
    /// The level of the last log entry, which its continuation lines share.
    Level(Level),
}

/// The level of a log entry.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Level {
    Debug,
    Info,
    Warning,
    Error,
}

/// The coloured spans of one line, as byte ranges into it.
pub type Spans = Vec<(Range<usize>, Token)>;

/// Colours `line`, which follows a line that left `carry` open. The spans are pushed onto `spans` in order, and what
/// this line leaves open is returned.
pub fn lex(language: Language, carry: Carry, line: &str, spans: &mut Spans) -> Carry {
    match language {
        Language::Plain => Carry::None,
        Language::Json => json::lex(carry, line, spans),
        Language::Toml => toml::lex(carry, line, spans),
        Language::Yaml => yaml::lex(carry, line, spans),
        Language::Ini => {
            ini::lex(line, spans);
            Carry::None
        }
        Language::Xml => xml::lex(carry, line, spans),
        Language::Log => log::lex(carry, line, spans),
        Language::Crash => {
            crash::lex(line, spans);
            Carry::None
        }
        Language::Function => {
            function::lex(line, spans);
            Carry::None
        }
        Language::Script => script::lex(carry, line, spans),
    }
}

/// What the highlighter is set up with. A new `revision` colours the whole text again, which a text replaced from
/// outside the editor (an undo, a reload) needs.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Settings {
    pub language: Language,
    pub revision: u64,
}

/// The highlighter iced's text editor drives: it is asked for one line after another, from the line it says it is at.
#[derive(Debug)]
pub struct Highlighter {
    language: Language,
    /// What each line was handed by the one before it. `carries[n]` belongs to line `n`.
    carries: Vec<Carry>,
    /// The next line to colour.
    current: usize,
}

impl iced::advanced::text::Highlighter for Highlighter {
    type Settings = Settings;
    type Highlight = Token;
    type Iterator<'a> = std::vec::IntoIter<(Range<usize>, Token)>;

    fn new(settings: &Self::Settings) -> Self {
        Self {
            language: settings.language,
            carries: Vec::new(),
            current: 0,
        }
    }

    fn update(&mut self, new_settings: &Self::Settings) {
        self.language = new_settings.language;
        self.carries.clear();
        self.current = 0;
    }

    fn change_line(&mut self, line: usize) {
        // What `line` was handed still stands, since the lines before it did not change. Everything after it is
        // worked out again as the lines are coloured.
        self.current = self.current.min(line);
        self.carries.truncate(self.current + 1);
    }

    fn highlight_line(&mut self, line: &str) -> Self::Iterator<'_> {
        let carry = self.carries.get(self.current).copied().unwrap_or_default();
        let mut spans = Spans::new();
        let next = lex(self.language, carry, line, &mut spans);
        self.carries.truncate(self.current + 1);
        if self.carries.len() <= self.current {
            self.carries.resize(self.current + 1, Carry::None);
        }
        self.carries.push(next);
        self.current += 1;
        spans.into_iter()
    }

    fn current_line(&self) -> usize {
        self.current
    }
}

/// The colour of a token. The colours are the app's own, so the editor matches the Logs tab and the rest of the page.
pub fn format(token: &Token, _theme: &Theme) -> Format<Font> {
    let color = match token {
        Token::Comment | Token::Faint => color::text_placeholder(),
        Token::Key | Token::Tag | Token::Debug => color::cyan(),
        Token::Text | Token::Info => color::primary(),
        Token::Number | Token::Attribute | Token::Warning => color::amber(),
        Token::Constant | Token::Keyword => color::violet_light(),
        Token::Punctuation => color::text_muted(),
        Token::Section | Token::Error => color::pink(),
    };
    Format {
        color: Some(color),
        font: None,
    }
}

/// Where the string that opens at `start` ends: the index after its closing `quote`, and whether it closed on this
/// line. A backslash keeps the character after it from closing the string.
fn string_end(bytes: &[u8], start: usize, quote: u8) -> (usize, bool) {
    string_rest(bytes, start + 1, quote)
}

/// Where a string already open before `from` ends: the index after its closing `quote`, and whether it closed on this
/// line. A string carried over from the line above is read with `from` at zero.
fn string_rest(bytes: &[u8], from: usize, quote: u8) -> (usize, bool) {
    let mut index = from;
    while index < bytes.len() {
        match bytes[index] {
            b'\\' => index += 2,
            byte if byte == quote => return (index + 1, true),
            _ => index += 1,
        }
    }
    (bytes.len(), false)
}

/// Where the run of bytes that pass `keep` ends, starting at `start`.
fn run_end(bytes: &[u8], start: usize, keep: impl Fn(u8) -> bool) -> usize {
    bytes[start..]
        .iter()
        .position(|byte| !keep(*byte))
        .map_or(bytes.len(), |offset| start + offset)
}

/// The index of the first byte at or after `start` that is not a space or a tab.
fn skip_spaces(bytes: &[u8], start: usize) -> usize {
    run_end(bytes, start, |byte| byte == b' ' || byte == b'\t')
}

/// Whether a byte can be part of a bare word: a name, a number, a path.
fn is_word(byte: u8) -> bool {
    byte.is_ascii_alphanumeric() || matches!(byte, b'_' | b'-' | b'.' | b'+') || byte >= 0x80
}

/// Whether `word` reads as a number: decimal, hexadecimal, with a fraction, an exponent, digit separators, or one of
/// the type letters NBT puts after a number (`1b`, `0.5f`). Dates and times in TOML count as numbers too.
fn is_number(word: &str) -> bool {
    let digits = word.strip_prefix(['-', '+']).unwrap_or(word);
    let Some(first) = digits.bytes().next() else {
        return false;
    };
    if !first.is_ascii_digit() && !(first == b'.' && digits.len() > 1) {
        return matches!(digits, "inf" | "nan" | "Infinity" | "NaN");
    }
    if let Some(hex) = digits.strip_prefix("0x").or_else(|| digits.strip_prefix("0X")) {
        return !hex.is_empty() && hex.bytes().all(|byte| byte.is_ascii_hexdigit() || byte == b'_');
    }
    let body = digits.trim_end_matches(['b', 'B', 's', 'S', 'l', 'L', 'f', 'F', 'd', 'D']);
    !body.is_empty()
        && body
            .bytes()
            .all(|byte| byte.is_ascii_digit() || matches!(byte, b'.' | b'_' | b'e' | b'E' | b'-' | b'+' | b':' | b'T' | b'Z'))
}

/// The token of a bare word used as a value: a number, a constant, or nothing when it is just text.
fn value_token(word: &str) -> Option<Token> {
    if matches!(
        word,
        "true" | "false" | "null" | "yes" | "no" | "on" | "off" | "~" | "True" | "False" | "TRUE" | "FALSE"
    ) {
        Some(Token::Constant)
    } else if is_number(word) {
        Some(Token::Number)
    } else {
        None
    }
}

/// Colours a block comment that `carry` left open, up to and including its `*/`. Returns where the comment ends on this
/// line, or `None` when the whole line is comment and the next line still is.
fn close_block_comment(line: &str, spans: &mut Spans) -> Option<usize> {
    match line.find("*/") {
        Some(end) => {
            spans.push((0..end + 2, Token::Comment));
            Some(end + 2)
        }
        None => {
            if !line.is_empty() {
                spans.push((0..line.len(), Token::Comment));
            }
            None
        }
    }
}

#[cfg(test)]
#[path = "../../../../../tests/app/components/editor/highlight/mod.rs"]
mod tests;
