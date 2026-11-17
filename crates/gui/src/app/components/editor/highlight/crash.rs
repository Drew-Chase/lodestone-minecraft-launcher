//! Crash reports: the banner, the headings of each part, the `Name: value` details, the exception that was thrown, and
//! the stack frames under it.

use super::log::is_exception;
use super::{Spans, Token};

pub fn lex(line: &str, spans: &mut Spans) {
    let text = line.trim_start();
    if text.is_empty() {
        return;
    }
    let start = line.len() - text.len();
    let whole = start..line.len();
    // `---- Minecraft Crash Report ----`, and the heading of a part: `-- System Details --`.
    if text.starts_with("--") && text.trim_end().ends_with("--") {
        spans.push((whole, Token::Section));
    } else if text.starts_with("//") {
        spans.push((whole, Token::Comment));
    } else if text.starts_with("at ") || text.starts_with("... ") {
        frame(line, start, spans);
    } else if is_exception(text) {
        spans.push((whole, Token::Error));
    } else if let Some(colon) = detail_colon(text) {
        // `Time: 2026-01-02`, `Description: Ticking entity`, and the indented details under a heading.
        spans.push((start..start + colon, Token::Key));
        spans.push((start + colon..start + colon + 1, Token::Punctuation));
    }
}

/// Colours a stack frame: faint, with the file and line it names in the colour of a link, since that is the part to
/// read.
fn frame(line: &str, start: usize, spans: &mut Spans) {
    match (line.rfind('('), line.rfind(')')) {
        (Some(open), Some(close)) if open < close && open > start => {
            spans.push((start..open + 1, Token::Faint));
            spans.push((open + 1..close, Token::Tag));
            spans.push((close..line.len(), Token::Faint));
        }
        _ => spans.push((start..line.len(), Token::Faint)),
    }
}

/// Where the colon that ends a detail's name is in `text`: a short name, starting with a capital letter or a word, that
/// a space or the end of the line follows.
fn detail_colon(text: &str) -> Option<usize> {
    let colon = text.find(':')?;
    let name = &text[..colon];
    let follows = text.as_bytes().get(colon + 1);
    let named = !name.is_empty()
        && name.len() <= 48
        && name.chars().next().is_some_and(char::is_alphanumeric)
        && name
            .chars()
            .all(|character| character.is_alphanumeric() || matches!(character, ' ' | '_' | '-' | '.' | '(' | ')' | '\'' | '/' | '#'));
    (named && matches!(follows, None | Some(b' ' | b'\t'))).then_some(colon)
}

#[cfg(test)]
#[path = "../../../../../tests/app/components/editor/highlight/crash.rs"]
mod tests;
