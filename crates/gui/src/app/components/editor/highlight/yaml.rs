//! YAML, as Paper, Bukkit plugins and some mods write their configs: mappings, lists, quoted and block strings,
//! anchors and comments.

use super::{Carry, Spans, Token, is_word, run_end, skip_spaces, string_end, value_token};

pub fn lex(carry: Carry, line: &str, spans: &mut Spans) -> Carry {
    let bytes = line.as_bytes();
    let indent = skip_spaces(bytes, 0);
    if let Carry::Block(depth) = carry {
        // A block string runs for as long as its lines are indented deeper than the key that opened it. An empty line
        // is part of it too.
        if indent == bytes.len() {
            return carry;
        }
        if indent > depth {
            spans.push((indent..bytes.len(), Token::Text));
            return carry;
        }
    }
    let mut index = indent;
    if line[index..].starts_with("---") || line[index..].starts_with("...") {
        spans.push((index..index + 3, Token::Punctuation));
        index += 3;
    }
    // The dashes of list items, which may nest: `- - a`.
    while bytes.get(index) == Some(&b'-') && matches!(bytes.get(index + 1), None | Some(b' ')) {
        spans.push((index..index + 1, Token::Punctuation));
        index = skip_spaces(bytes, index + 1);
    }
    // A key ends at a colon that is followed by a space or the end of the line.
    if let Some(colon) = key_colon(bytes, index) {
        let end = line[index..colon].trim_end().len() + index;
        spans.push((index..end, Token::Key));
        spans.push((colon..colon + 1, Token::Punctuation));
        index = colon + 1;
    }
    while index < bytes.len() {
        let byte = bytes[index];
        match byte {
            b' ' | b'\t' => index += 1,
            // A hash starts a comment only at the start of the value or after a space.
            b'#' if index == 0 || matches!(bytes[index - 1], b' ' | b'\t') => {
                spans.push((index..bytes.len(), Token::Comment));
                return Carry::None;
            }
            b'"' | b'\'' => {
                let (end, _) = string_end(bytes, index, byte);
                spans.push((index..end, Token::Text));
                index = end;
            }
            b'|' | b'>' if opens_block(bytes, index) => {
                spans.push((index..bytes.len(), Token::Punctuation));
                return Carry::Block(indent);
            }
            b'&' | b'*' | b'!' if bytes.get(index + 1).is_some_and(|next| is_word(*next) || *next == b'!') => {
                let end = run_end(bytes, index + 1, |byte| is_word(byte) || byte == b'!');
                spans.push((index..end, Token::Tag));
                index = end;
            }
            b'[' | b']' | b'{' | b'}' | b',' => {
                spans.push((index..index + 1, Token::Punctuation));
                index += 1;
            }
            _ if is_word(byte) || byte == b'~' => {
                let end = run_end(bytes, index, |byte| is_word(byte) || byte == b'~' || byte == b':');
                // Only a word that is the whole value is a number or a constant: `1.20.1 release` is text.
                let alone = matches!(bytes.get(skip_spaces(bytes, end)), None | Some(b'#' | b',' | b']' | b'}'));
                if let Some(token) = value_token(&line[index..end]).filter(|_| alone) {
                    spans.push((index..end, token));
                }
                index = end;
            }
            _ => index += 1,
        }
    }
    Carry::None
}

/// The colon that ends the key starting at `start`, when the line has one: a colon outside quotes that a space or the
/// end of the line follows. A value that only holds a colon, such as a time or a URL, has none.
fn key_colon(bytes: &[u8], start: usize) -> Option<usize> {
    let mut index = start;
    while index < bytes.len() {
        match bytes[index] {
            quote @ (b'"' | b'\'') if index == start => index = string_end(bytes, index, quote).0,
            b':' if matches!(bytes.get(index + 1), None | Some(b' ' | b'\t')) => return (index > start).then_some(index),
            b'#' if index > start && bytes[index - 1] == b' ' => return None,
            b'[' | b'{' if index == start => return None,
            _ => index += 1,
        }
    }
    None
}

/// Whether the `|` or `>` at `index` opens a block string: nothing follows it on the line but its chomping and indent
/// marks and a comment.
fn opens_block(bytes: &[u8], index: usize) -> bool {
    let after = run_end(bytes, index + 1, |byte| matches!(byte, b'+' | b'-') || byte.is_ascii_digit());
    matches!(bytes.get(skip_spaces(bytes, after)), None | Some(b'#'))
}

#[cfg(test)]
#[path = "../../../../../tests/app/components/editor/highlight/yaml.rs"]
mod tests;
