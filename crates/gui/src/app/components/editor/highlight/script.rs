//! Scripts with C-style syntax: the JavaScript of KubeJS, the ZenScript of CraftTweaker, and the Java, Kotlin, Groovy
//! and GLSL a pack may carry. They share their comments, strings, numbers and braces, and the keywords are the union
//! of what those languages reserve.

use super::{Carry, Spans, Token, close_block_comment, is_number, run_end, skip_spaces, string_end, string_rest};

/// The reserved words of the languages this lexer covers.
const KEYWORDS: &[&str] = &[
    "abstract",
    "as",
    "async",
    "await",
    "boolean",
    "break",
    "byte",
    "case",
    "catch",
    "char",
    "class",
    "const",
    "continue",
    "default",
    "do",
    "double",
    "else",
    "enum",
    "export",
    "extends",
    "final",
    "finally",
    "float",
    "for",
    "fun",
    "function",
    "global",
    "if",
    "implements",
    "import",
    "in",
    "instanceof",
    "int",
    "interface",
    "let",
    "long",
    "new",
    "of",
    "package",
    "private",
    "protected",
    "public",
    "return",
    "short",
    "static",
    "string",
    "super",
    "switch",
    "this",
    "throw",
    "throws",
    "try",
    "typeof",
    "val",
    "var",
    "void",
    "while",
    "yield",
    "uniform",
    "varying",
    "attribute",
    "vec2",
    "vec3",
    "vec4",
    "mat3",
    "mat4",
    "sampler2D",
    "out",
    "inout",
    "def",
    "when",
    "object",
    "override",
    "is",
];
/// The words that stand for a fixed value.
const CONSTANTS: &[&str] = &["true", "false", "null", "undefined", "NaN"];

pub fn lex(carry: Carry, line: &str, spans: &mut Spans) -> Carry {
    let bytes = line.as_bytes();
    let mut index = 0;
    match carry {
        Carry::Comment => match close_block_comment(line, spans) {
            Some(end) => index = end,
            None => return Carry::Comment,
        },
        // A template string in backquotes may run over several lines.
        Carry::Text(quote) => {
            let (end, closed) = string_rest(bytes, 0, quote);
            if end > 0 {
                spans.push((0..end, Token::Text));
            }
            if !closed {
                return carry;
            }
            index = end;
        }
        _ => {}
    }
    // A directive takes its word: `#priority 10` in ZenScript, `#version 150` in GLSL.
    let first = skip_spaces(bytes, index);
    if index == 0 && bytes.get(first) == Some(&b'#') {
        let end = run_end(bytes, first + 1, |byte| byte.is_ascii_alphanumeric() || byte == b'_');
        spans.push((first..end, Token::Keyword));
        index = end;
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
            b'"' | b'\'' | b'`' => {
                let (end, closed) = string_end(bytes, index, byte);
                spans.push((index..end, Token::Text));
                if !closed && byte == b'`' {
                    return Carry::Text(byte);
                }
                index = end;
            }
            // A ZenScript bracket handler: `<item:minecraft:stone>`, `<tag:items:forge:ingots>`.
            b'<' if is_bracket_handler(bytes, index) => {
                let end = bytes[index..]
                    .iter()
                    .position(|byte| *byte == b'>')
                    .map_or(bytes.len(), |close| index + close + 1);
                spans.push((index..end, Token::Tag));
                index = end;
            }
            b'{' | b'}' | b'(' | b')' | b'[' | b']' | b';' | b',' => {
                spans.push((index..index + 1, Token::Punctuation));
                index += 1;
            }
            _ if byte.is_ascii_alphanumeric() || byte == b'_' || byte == b'$' || byte >= 0x80 => {
                let end = run_end(bytes, index, |byte| {
                    byte.is_ascii_alphanumeric() || matches!(byte, b'_' | b'$' | b'.') || byte >= 0x80
                });
                let word = &line[index..end];
                // A dotted name is a path through objects, and only a number keeps its dot.
                let (word, end) = match word.find('.') {
                    Some(dot) if !is_number(word) => (&word[..dot], index + dot),
                    _ => (word, end),
                };
                let token = if KEYWORDS.contains(&word) {
                    Some(Token::Keyword)
                } else if CONSTANTS.contains(&word) {
                    Some(Token::Constant)
                } else if is_number(word) {
                    Some(Token::Number)
                } else {
                    None
                };
                if let Some(token) = token {
                    spans.push((index..end, token));
                }
                index = end.max(index + 1);
            }
            _ => index += 1,
        }
    }
    Carry::None
}

/// Whether the `<` at `index` opens a bracket handler: a lower-case word and a colon follow it, and a `>` closes it on
/// this line. A less-than sign has a space or a number after it.
fn is_bracket_handler(bytes: &[u8], index: usize) -> bool {
    let word_end = run_end(bytes, index + 1, |byte| byte.is_ascii_lowercase());
    word_end > index + 1 && bytes.get(word_end) == Some(&b':') && bytes[word_end..].contains(&b'>')
}

#[cfg(test)]
#[path = "../../../../../tests/app/components/editor/highlight/script.rs"]
mod tests;
