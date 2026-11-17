//! TOML, the format of most Forge, NeoForge and Fabric configs: tables, dotted keys, the four kinds of string, dates
//! and comments.

use super::{Carry, Spans, Token, is_word, run_end, skip_spaces, string_end, value_token};

pub fn lex(carry: Carry, line: &str, spans: &mut Spans) -> Carry {
    let bytes = line.as_bytes();
    let mut index = 0;
    if let Carry::Text(quote) = carry {
        match triple_end(bytes, 0, quote) {
            Some(end) => {
                spans.push((0..end, Token::Text));
                index = end;
            }
            None => {
                if !line.is_empty() {
                    spans.push((0..bytes.len(), Token::Text));
                }
                return carry;
            }
        }
    }
    // A table header takes the line: `[servers.alpha]` or `[[mods]]`.
    let first = skip_spaces(bytes, index);
    if index == 0 && bytes.get(first) == Some(&b'[') && is_header(&line[first..]) {
        let close = line.rfind(']').map_or(bytes.len(), |close| close + 1);
        spans.push((first..close, Token::Section));
        index = close;
    }
    while index < bytes.len() {
        let byte = bytes[index];
        match byte {
            b' ' | b'\t' => index += 1,
            b'#' => {
                spans.push((index..bytes.len(), Token::Comment));
                return Carry::None;
            }
            b'"' | b'\'' if bytes[index..].starts_with(&[byte; 3]) => match triple_end(bytes, index + 3, byte) {
                Some(end) => {
                    spans.push((index..end, Token::Text));
                    index = end;
                }
                None => {
                    spans.push((index..bytes.len(), Token::Text));
                    return Carry::Text(byte);
                }
            },
            b'"' | b'\'' => {
                // A literal string in single quotes has no escapes, so a backslash does not hide its closing quote.
                let end = if byte == b'\'' {
                    bytes[index + 1..]
                        .iter()
                        .position(|next| *next == b'\'')
                        .map_or(bytes.len(), |close| index + close + 2)
                } else {
                    string_end(bytes, index, byte).0
                };
                spans.push((index..end, key_or(bytes, end, Token::Text)));
                index = end;
            }
            b'=' | b',' | b'[' | b']' | b'{' | b'}' => {
                spans.push((index..index + 1, Token::Punctuation));
                index += 1;
            }
            _ if is_word(byte) || byte == b':' => {
                let end = run_end(bytes, index, |byte| is_word(byte) || byte == b':');
                let word = &line[index..end];
                if key_or(bytes, end, Token::Text) == Token::Key {
                    spans.push((index..end, Token::Key));
                } else if let Some(token) = value_token(word) {
                    spans.push((index..end, token));
                }
                index = end;
            }
            _ => index += 1,
        }
    }
    Carry::None
}

/// Whether `text`, which starts with `[`, is a table header: brackets around a name, with nothing after but a comment.
/// An array value that opens a line of a multi-line array is not one.
fn is_header(text: &str) -> bool {
    let code = text.split('#').next().unwrap_or(text).trim_end();
    let inner = code.trim_start_matches('[').trim_end_matches(']');
    code.ends_with(']') && !inner.trim().is_empty() && !inner.contains(['[', ']', ',', '='])
}

/// Where the multi-line string ends when its closing three `quote`s are on this line, searching from `start`.
fn triple_end(bytes: &[u8], start: usize, quote: u8) -> Option<usize> {
    let close = [quote; 3];
    let mut index = start;
    while index + 3 <= bytes.len() {
        if quote == b'"' && bytes[index] == b'\\' {
            index += 2;
        } else if bytes[index..].starts_with(&close) {
            return Some(index + 3);
        } else {
            index += 1;
        }
    }
    None
}

/// `Token::Key` when an equals sign follows the text that ends at `end`, which makes that text a key. Otherwise `other`.
fn key_or(bytes: &[u8], end: usize, other: Token) -> Token {
    if bytes.get(skip_spaces(bytes, end)) == Some(&b'=') {
        Token::Key
    } else {
        other
    }
}

#[cfg(test)]
#[path = "../../../../../tests/app/components/editor/highlight/toml.rs"]
mod tests;
