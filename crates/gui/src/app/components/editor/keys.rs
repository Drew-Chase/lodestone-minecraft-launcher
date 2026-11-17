//! The editor's typing helpers: brackets and quotes that close themselves, an Enter that keeps the indent, and a Tab
//! that indents by the file's own unit.
//!
//! [`plan`] reads what was typed and what is around the caret, and answers with the edits to make in its place, or
//! with nothing when the key should do what it always does. It touches no editor, so every rule is tested as plain
//! text.

use super::language::Language;

/// What was typed.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Typed {
    Char(char),
    Enter,
    Backspace,
    Tab,
}

/// What is around the caret when a key is typed.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Context<'a> {
    /// The text of the line the caret is on.
    pub line: &'a str,
    /// Where the caret is in the line, as a byte index.
    pub column: usize,
    /// Whether text is selected.
    pub selection: bool,
    pub language: Language,
    /// One level of indent: a tab, or a run of spaces.
    pub unit: &'a str,
}

/// One edit to make in place of the key.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Step {
    /// Inserts text at the caret, as a paste does. It may hold line breaks.
    Text(String),
    /// Deletes the character before the caret.
    Backspace,
    /// Deletes the character after the caret.
    Delete,
    /// Moves the caret one character left.
    Left,
    /// Moves the caret one character right.
    Right,
    /// Puts `open` before the selection and `close` after it.
    Wrap { open: char, close: char },
    /// Indents every selected line by one level.
    IndentLines,
}

/// The character that closes `open`, when `open` opens a pair.
pub fn closer(open: char) -> Option<char> {
    match open {
        '(' => Some(')'),
        '[' => Some(']'),
        '{' => Some('}'),
        '"' | '\'' | '`' => Some(open),
        _ => None,
    }
}

/// Whether `character` is a quote, which opens and closes with the same mark.
fn is_quote(character: char) -> bool {
    matches!(character, '"' | '\'' | '`')
}

/// Whether a pair may be opened before `next`, the character after the caret: at the end of the line, before a space,
/// and before something that closes or separates. In front of a word, the opener is typed alone.
fn room_for_pair(next: Option<char>) -> bool {
    next.is_none_or(|next| next.is_whitespace() || matches!(next, ')' | ']' | '}' | ',' | ';' | ':' | '>' | '=' | '"' | '\'' | '`'))
}

/// The edits to make in place of `typed`, or `None` when the key keeps its usual meaning.
pub fn plan(typed: Typed, context: &Context<'_>) -> Option<Vec<Step>> {
    let column = context.column.min(context.line.len());
    let (before, after) = context.line.split_at(column);
    let previous = before.chars().next_back();
    let next = after.chars().next();
    match typed {
        Typed::Char(_) if !context.language.pairs() => None,
        Typed::Char(open) if context.selection => closer(open).map(|close| vec![Step::Wrap { open, close }]),
        // Typing the mark that already stands after the caret steps over it.
        Typed::Char(close) if next == Some(close) && (matches!(close, ')' | ']' | '}') || is_quote(close)) => Some(vec![Step::Right]),
        Typed::Char(open) => {
            let close = closer(open)?;
            // A quote after a letter is an apostrophe or the end of a string, and one after a backslash is escaped.
            let inside_word = is_quote(open) && previous.is_some_and(|previous| previous.is_alphanumeric() || previous == '\\' || previous == open);
            (room_for_pair(next) && !inside_word).then(|| vec![Step::Text(format!("{open}{close}")), Step::Left])
        }
        Typed::Backspace if context.selection => None,
        Typed::Backspace => {
            // Between an empty pair, both marks go.
            if previous.is_some_and(|previous| closer(previous) == next && next.is_some()) && context.language.pairs() {
                return Some(vec![Step::Delete, Step::Backspace]);
            }
            // In the indent, Backspace goes back one level, not one space.
            let width = context.unit.len();
            let spaces = context.unit.starts_with(' ') && !before.is_empty() && before.bytes().all(|byte| byte == b' ');
            (spaces && width > 1).then(|| {
                let back = (before.len() - 1) % width + 1;
                vec![Step::Backspace; back]
            })
        }
        Typed::Enter => {
            let indent: String = context.line.chars().take_while(|character| matches!(character, ' ' | '\t')).collect();
            // The indent of a caret that sits inside the indent is only what is before it.
            let indent = if indent.len() > column { indent[..column].to_string() } else { indent };
            let opener = before.trim_end().chars().next_back();
            let opens = opener.is_some_and(|opener| matches!(opener, '{' | '[' | '(') || (opener == ':' && context.language.colon_opens_block()));
            let closes = opener.and_then(closer).is_some_and(|close| after.trim_start().starts_with(close)) && !opener.is_some_and(is_quote);
            if opens && closes {
                // Between a pair, the closer goes to its own line and the caret to an indented line between them.
                let back = indent.chars().count() + 1;
                let mut steps = vec![Step::Text(format!("\n{indent}{}\n{indent}", context.unit))];
                steps.extend(std::iter::repeat_n(Step::Left, back));
                Some(steps)
            } else if opens {
                Some(vec![Step::Text(format!("\n{indent}{}", context.unit))])
            } else if !indent.is_empty() {
                Some(vec![Step::Text(format!("\n{indent}"))])
            } else {
                None
            }
        }
        Typed::Tab if context.selection => Some(vec![Step::IndentLines]),
        Typed::Tab => {
            // Spaces go up to the next level, so a Tab in the middle of a level lines up with the rest.
            let fill = if context.unit.starts_with(' ') {
                let width = context.unit.len();
                " ".repeat(width - before.chars().count() % width)
            } else {
                context.unit.to_string()
            };
            Some(vec![Step::Text(fill)])
        }
    }
}

/// One level of indent for a file, read from its lines: a tab when its indented lines start with one, or as many spaces
/// as its shallowest indented line has. A file with no indented line takes the language's own width.
pub fn indent_unit<'a>(lines: impl Iterator<Item = &'a str>, language: Language) -> String {
    let mut narrowest: Option<usize> = None;
    for line in lines.take(2000) {
        if line.starts_with('\t') {
            return "\t".to_string();
        }
        let spaces = line.bytes().take_while(|byte| *byte == b' ').count();
        if spaces >= 2 && spaces < line.len() {
            narrowest = Some(narrowest.map_or(spaces, |narrowest| narrowest.min(spaces)));
        }
    }
    " ".repeat(narrowest.filter(|width| *width <= 8).unwrap_or_else(|| language.indent_width()))
}

/// How many characters to remove from the start of `line` to take one level of indent off it: a tab, or up to one
/// unit of spaces.
pub fn unindent_width(line: &str, unit: &str) -> usize {
    if line.starts_with('\t') {
        1
    } else {
        line.bytes().take_while(|byte| *byte == b' ').count().min(unit.len().max(1))
    }
}

#[cfg(test)]
#[path = "../../../../tests/app/components/editor/keys.rs"]
mod tests;
