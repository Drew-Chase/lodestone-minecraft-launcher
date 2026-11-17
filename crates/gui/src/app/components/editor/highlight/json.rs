//! JSON, and the looser forms Minecraft files use: JSON5 and JSONC with comments, single quotes and bare keys, and
//! stringified NBT with its typed numbers.

use super::{Carry, Spans, Token, close_block_comment, is_word, run_end, skip_spaces, string_end, value_token};

pub fn lex(carry: Carry, line: &str, spans: &mut Spans) -> Carry {
    let bytes = line.as_bytes();
    let mut index = 0;
    if carry == Carry::Comment {
        match close_block_comment(line, spans) {
            Some(end) => index = end,
            None => return Carry::Comment,
        }
    }
    while index < bytes.len() {
        let byte = bytes[index];
        match byte {
            b' ' | b'\t' => index += 1,
            b'/' if bytes.get(index + 1) == Some(&b'/') => {
                spans.push((index..bytes.len(), Token::Comment));
                return Carry::None;
            }
            b'/' if bytes.get(index + 1) == Some(&b'*') => match line[index + 2..].find("*/") {
                Some(end) => {
                    let end = index + 2 + end + 2;
                    spans.push((index..end, Token::Comment));
                    index = end;
                }
                None => {
                    spans.push((index..bytes.len(), Token::Comment));
                    return Carry::Comment;
                }
            },
            b'"' | b'\'' => {
                let (end, _) = string_end(bytes, index, byte);
                spans.push((index..end, name_or(bytes, end, Token::Text)));
                index = end;
            }
            b'{' | b'}' | b'[' | b']' | b',' | b':' | b';' => {
                spans.push((index..index + 1, Token::Punctuation));
                index += 1;
            }
            _ if is_word(byte) => {
                let end = run_end(bytes, index, is_word);
                let word = &line[index..end];
                // A bare word before a colon is a key, as JSON5 and NBT write them. A number or a constant is a value.
                let token = match value_token(word) {
                    Some(value) => Some(name_or(bytes, end, value)),
                    None => (name_or(bytes, end, Token::Text) == Token::Key).then_some(Token::Key),
                };
                if let Some(token) = token {
                    spans.push((index..end, token));
                }
                index = end;
            }
            _ => index += 1,
        }
    }
    Carry::None
}

/// `Token::Key` when a colon follows the text that ends at `end`, which makes that text a key. Otherwise `other`.
fn name_or(bytes: &[u8], end: usize, other: Token) -> Token {
    if bytes.get(skip_spaces(bytes, end)) == Some(&b':') {
        Token::Key
    } else {
        other
    }
}

#[cfg(test)]
#[path = "../../../../../tests/app/components/editor/highlight/json.rs"]
mod tests;
