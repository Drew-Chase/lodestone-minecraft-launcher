//! XML, which the game's logging configs and some mod configs are written in: tags, attributes, quoted values and
//! comments.

use super::{Carry, Spans, Token, run_end, skip_spaces};

pub fn lex(carry: Carry, line: &str, spans: &mut Spans) -> Carry {
    let bytes = line.as_bytes();
    let mut index = 0;
    match carry {
        Carry::Comment => match line.find("-->") {
            Some(end) => {
                spans.push((0..end + 3, Token::Comment));
                index = end + 3;
            }
            None => {
                if !line.is_empty() {
                    spans.push((0..bytes.len(), Token::Comment));
                }
                return Carry::Comment;
            }
        },
        Carry::Tag => match attributes(bytes, 0, spans) {
            Some(end) => index = end,
            None => return Carry::Tag,
        },
        _ => {}
    }
    while index < bytes.len() {
        if line[index..].starts_with("<!--") {
            match line[index + 4..].find("-->") {
                Some(end) => {
                    let end = index + 4 + end + 3;
                    spans.push((index..end, Token::Comment));
                    index = end;
                }
                None => {
                    spans.push((index..bytes.len(), Token::Comment));
                    return Carry::Comment;
                }
            }
        } else if bytes[index] == b'<' {
            // The bracket with its `/`, `?` or `!`, then the tag's name, then its attributes.
            let name_start = run_end(bytes, index + 1, |byte| matches!(byte, b'/' | b'?' | b'!'));
            let name_end = run_end(bytes, name_start, is_name);
            spans.push((index..name_start, Token::Punctuation));
            if name_end > name_start {
                spans.push((name_start..name_end, Token::Tag));
            }
            match attributes(bytes, name_end, spans) {
                Some(end) => index = end,
                None => return Carry::Tag,
            }
        } else {
            index += 1;
        }
    }
    Carry::None
}

/// Colours the attributes of a tag from `start` to the `>` that closes it. Returns the index after the `>`, or `None`
/// when the tag goes on to the next line.
fn attributes(bytes: &[u8], start: usize, spans: &mut Spans) -> Option<usize> {
    let mut index = start;
    while index < bytes.len() {
        let byte = bytes[index];
        match byte {
            b'>' => {
                spans.push((index..index + 1, Token::Punctuation));
                return Some(index + 1);
            }
            b'/' | b'?' | b'=' => {
                spans.push((index..index + 1, Token::Punctuation));
                index += 1;
            }
            b'"' | b'\'' => {
                // A value has no escapes of its own, so only its own quote ends it.
                let end = bytes[index + 1..]
                    .iter()
                    .position(|next| *next == byte)
                    .map_or(bytes.len(), |close| index + close + 2);
                spans.push((index..end, Token::Text));
                index = end;
            }
            _ if is_name(byte) => {
                let end = run_end(bytes, index, is_name);
                spans.push((index..end, Token::Attribute));
                index = end;
            }
            _ => index = skip_spaces(bytes, index).max(index + 1),
        }
    }
    None
}

/// Whether a byte can be part of a tag's or an attribute's name.
fn is_name(byte: u8) -> bool {
    byte.is_ascii_alphanumeric() || matches!(byte, b'_' | b'-' | b'.' | b':') || byte >= 0x80
}

#[cfg(test)]
#[path = "../../../../../tests/app/components/editor/highlight/xml.rs"]
mod tests;
