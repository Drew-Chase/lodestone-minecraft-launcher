//! Game and launcher logs: the time and the level of each entry, with warnings and errors in their colours, as the Logs
//! tab draws them. A stack trace under an entry is drawn faint, and its exception lines as errors.

use super::{Carry, Level, Spans, Token};

/// The level names a log line may carry, and the level each one counts as.
const LEVELS: &[(&str, Level)] = &[
    ("ERROR", Level::Error),
    ("FATAL", Level::Error),
    ("SEVERE", Level::Error),
    ("WARNING", Level::Warning),
    ("WARN", Level::Warning),
    ("INFO", Level::Info),
    ("DEBUG", Level::Debug),
    ("TRACE", Level::Debug),
    ("FINE", Level::Debug),
];

/// How far into a line a level mark is looked for. A level named in the message itself is not the entry's level.
const PREFIX: usize = 96;

pub fn lex(carry: Carry, line: &str, spans: &mut Spans) -> Carry {
    let Some((mark, level)) = level_mark(line) else {
        return continuation(carry, line, spans);
    };
    // The time, and whatever else comes before the level, is faint. The bracket that names the level takes the level's
    // colour, and the message of a warning or an error takes it too.
    let bracket = line[..mark.start].rfind('[').unwrap_or(mark.start);
    let close = line[mark.end..].find(']').map_or(mark.end, |close| mark.end + close + 1);
    if bracket > 0 {
        spans.push((0..bracket, Token::Faint));
    }
    spans.push((bracket..close, token(level)));
    if matches!(level, Level::Warning | Level::Error) && close < line.len() {
        spans.push((close..line.len(), token(level)));
    }
    Carry::Level(level)
}

/// Colours a line with no level of its own, which belongs to the entry above it.
fn continuation(carry: Carry, line: &str, spans: &mut Spans) -> Carry {
    let text = line.trim_start();
    if text.is_empty() {
        return carry;
    }
    let start = line.len() - text.len();
    let token = if text.starts_with("at ") || text.starts_with("... ") {
        Some(Token::Faint)
    } else if is_exception(text) {
        Some(Token::Error)
    } else {
        match carry {
            Carry::Level(level @ (Level::Warning | Level::Error)) => Some(self::token(level)),
            _ => None,
        }
    };
    if let Some(token) = token {
        spans.push((start..line.len(), token));
    }
    carry
}

/// Whether `text` is the line of a stack trace that names what was thrown: `java.lang.IllegalStateException: …`, or a
/// `Caused by:` line.
pub fn is_exception(text: &str) -> bool {
    if text.starts_with("Caused by:") || text.starts_with("Exception in thread") {
        return true;
    }
    let class = text.split([':', ' ']).next().unwrap_or(text);
    class.contains('.') && (class.ends_with("Exception") || class.ends_with("Error") || class.ends_with("Throwable"))
}

/// The colour of a level.
fn token(level: Level) -> Token {
    match level {
        Level::Debug => Token::Debug,
        Level::Info => Token::Info,
        Level::Warning => Token::Warning,
        Level::Error => Token::Error,
    }
}

/// Where the line names its level, and which level it is. The name has to stand as a word near the start of the line:
/// `[main/INFO]`, `[INFO]`, ` WARN `.
fn level_mark(line: &str) -> Option<(std::ops::Range<usize>, Level)> {
    let mut end = line.len().min(PREFIX);
    while !line.is_char_boundary(end) {
        end -= 1;
    }
    let prefix = &line[..end];
    LEVELS
        .iter()
        .filter_map(|(name, level)| {
            let start = prefix
                .match_indices(name)
                .map(|(start, _)| start)
                .find(|start| stands_alone(prefix, *start, name.len()))?;
            Some((start..start + name.len(), *level))
        })
        .min_by_key(|(range, _)| range.start)
}

/// Whether the `length` bytes at `start` are a word on their own: not part of a longer word such as `INFORMATION`.
fn stands_alone(text: &str, start: usize, length: usize) -> bool {
    let bytes = text.as_bytes();
    let before = start.checked_sub(1).map(|index| bytes[index]);
    let after = bytes.get(start + length).copied();
    let boundary = |byte: Option<u8>| byte.is_none_or(|byte| !byte.is_ascii_alphanumeric());
    boundary(before) && boundary(after)
}

#[cfg(test)]
#[path = "../../../../../tests/app/components/editor/highlight/log.rs"]
mod tests;
