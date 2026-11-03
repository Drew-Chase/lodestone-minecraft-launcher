//! A server's description as plain text. The status describes its message of the day as a chat component: a string, a list
//! of components, or an object with its own text and the components it extends. Formatting codes, a section sign and the
//! character after it, are removed.

use serde_json::Value;

/// The plain text of a chat component, with its formatting codes removed.
pub fn plain(component: &Value) -> String {
    strip_codes(&flatten(component))
}

/// The text of a component and of everything it extends, with its formatting still in place.
fn flatten(component: &Value) -> String {
    match component {
        Value::String(text) => text.clone(),
        Value::Array(parts) => parts.iter().map(flatten).collect(),
        Value::Object(object) => {
            let mut text = object.get("text").map(flatten).unwrap_or_default();
            if let Some(extra) = object.get("extra") {
                text.push_str(&flatten(extra));
            }
            text
        }
        _ => String::new(),
    }
}

/// The text with its formatting codes removed: each section sign and the character after it.
pub fn strip_codes(text: &str) -> String {
    let mut out = String::with_capacity(text.len());
    let mut characters = text.chars();
    while let Some(character) = characters.next() {
        if character == '§' {
            characters.next();
        } else {
            out.push(character);
        }
    }
    out
}

#[cfg(test)]
#[path = "../tests/chat.rs"]
mod tests;
