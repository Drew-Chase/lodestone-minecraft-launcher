//! The `name=value` family: INI and `.conf` files, Java `.properties` and `.lang` files, the game's `options.txt`
//! with its colons, and the old Forge `.cfg` format with its typed names and braced categories.

use super::{Spans, Token, skip_spaces, value_token};

pub fn lex(line: &str, spans: &mut Spans) {
    let bytes = line.as_bytes();
    let start = skip_spaces(bytes, 0);
    let text = line[start..].trim_end();
    let end = start + text.len();
    if text.is_empty() {
        return;
    }
    if text.starts_with(['#', ';', '!']) || text.starts_with("//") {
        spans.push((start..end, Token::Comment));
        return;
    }
    // `[section]`, or a Forge category: a name, quoted or not, before an opening brace.
    if (text.starts_with('[') && text.ends_with(']')) || (text.ends_with('{') && !text.contains('=')) {
        spans.push((start..end, Token::Section));
        return;
    }
    if matches!(text, "}" | "<" | ">") {
        spans.push((start..end, Token::Punctuation));
        return;
    }
    // An old Forge config opens a list with its typed name and a `<`, and no equals sign: `S:blocks <`.
    if !text.contains('=')
        && let [mark, b':', ..] = text.as_bytes()
        && mark.is_ascii_uppercase()
        && let Some(name) = text.strip_suffix('<')
    {
        let name_end = start + name.trim_end().len();
        spans.push((start..start + 2, Token::Attribute));
        if name_end > start + 2 {
            spans.push((start + 2..name_end, Token::Key));
        }
        spans.push((end - 1..end, Token::Punctuation));
        return;
    }
    // The name ends at the first equals sign, or at the first colon when the line has no equals sign.
    let Some(separator) = text.find('=').or_else(|| text.find(':')) else {
        return;
    };
    let name = text[..separator].trim_end();
    let mut name_start = start;
    // An old Forge config marks each name with its type: `B:enabled=true`, `I:"max size"=4`.
    if text.as_bytes()[separator] == b'='
        && let [mark, b':', ..] = name.as_bytes()
        && mark.is_ascii_uppercase()
    {
        spans.push((start..start + 2, Token::Attribute));
        name_start += 2;
    }
    if start + name.len() > name_start {
        spans.push((name_start..start + name.len(), Token::Key));
    }
    spans.push((start + separator..start + separator + 1, Token::Punctuation));
    let value_start = skip_spaces(bytes, start + separator + 1);
    if value_start < end {
        let value = &line[value_start..end];
        // A list opened with `<` continues on the lines below it.
        let token = if value == "<" {
            Token::Punctuation
        } else {
            value_token(value).unwrap_or(Token::Text)
        };
        spans.push((value_start..end, token));
    }
}

#[cfg(test)]
#[path = "../../../../../tests/app/components/editor/highlight/ini.rs"]
mod tests;
