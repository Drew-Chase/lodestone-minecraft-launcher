//! Datapack functions: one command to a line, with selectors, coordinates, namespaced ids and NBT.

use super::{Spans, Token, is_number, is_word, run_end, skip_spaces, string_end};

/// The words that chain or branch a command, wherever they stand on the line.
const SUBCOMMANDS: &[&str] = &[
    "run",
    "as",
    "at",
    "if",
    "unless",
    "store",
    "positioned",
    "in",
    "facing",
    "rotated",
    "anchored",
    "align",
    "on",
    "summon",
    "result",
    "success",
];

pub fn lex(line: &str, spans: &mut Spans) {
    let bytes = line.as_bytes();
    let mut index = skip_spaces(bytes, 0);
    if bytes.get(index) == Some(&b'#') {
        spans.push((index..bytes.len(), Token::Comment));
        return;
    }
    // A macro line starts with a dollar sign, then its command.
    if bytes.get(index) == Some(&b'$') {
        spans.push((index..index + 1, Token::Punctuation));
        index += 1;
    }
    let mut first = true;
    while index < bytes.len() {
        let byte = bytes[index];
        match byte {
            b' ' | b'\t' => index += 1,
            b'"' | b'\'' => {
                let (end, _) = string_end(bytes, index, byte);
                spans.push((index..end, Token::Text));
                index = end;
            }
            b'@' => {
                let end = run_end(bytes, index + 1, |byte| byte.is_ascii_alphabetic());
                spans.push((index..end, Token::Constant));
                index = end;
            }
            // A relative or local coordinate: `~`, `~1`, `^-2.5`.
            b'~' | b'^' => {
                let end = run_end(bytes, index + 1, |byte| byte.is_ascii_digit() || matches!(byte, b'.' | b'-'));
                spans.push((index..end, Token::Number));
                index = end;
            }
            b'{' | b'}' | b'[' | b']' | b',' | b'=' | b'!' => {
                spans.push((index..index + 1, Token::Punctuation));
                index += 1;
            }
            _ if is_word(byte) || matches!(byte, b':' | b'/' | b'#') => {
                let end = run_end(bytes, index, |byte| is_word(byte) || matches!(byte, b':' | b'/' | b'#'));
                let word = &line[index..end];
                let token = if first || SUBCOMMANDS.contains(&word) {
                    Some(Token::Keyword)
                } else if is_number(word) || word == "true" || word == "false" {
                    Some(Token::Number)
                } else if word.contains(':') && !word.ends_with(':') {
                    // `minecraft:stone`, `#minecraft:logs`, `mypack:tick/main`.
                    Some(Token::Tag)
                } else if bytes.get(end) == Some(&b':') || word.ends_with(':') {
                    Some(Token::Key)
                } else {
                    None
                };
                if let Some(token) = token {
                    spans.push((index..end, token));
                }
                index = end;
            }
            _ => index += 1,
        }
        first = false;
    }
}

#[cfg(test)]
#[path = "../../../../../tests/app/components/editor/highlight/function.rs"]
mod tests;
