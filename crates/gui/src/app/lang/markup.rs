//! Inline markup for catalogue text:
//!
//! - `%P` starts primary (Lode Green) ink.
//! - `%C` clears back to the base style.
//! - `%%` is a literal `%`.
//!
//! Any other `%` is kept as written, so text such as `50% off` needs no escape.

/// A run of text and whether it is primary ink.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Segment<'a> {
    pub text: &'a str,
    pub primary: bool,
}

/// Splits `raw` into its runs, dropping empty ones.
pub fn segments(raw: &str) -> Vec<Segment<'_>> {
    let bytes = raw.as_bytes();
    let mut out = Vec::new();
    let mut primary = false;
    // Where the run being collected starts.
    let mut start = 0;
    let mut i = 0;
    while i < bytes.len() {
        if bytes[i] != b'%' || i + 1 == bytes.len() {
            i += 1;
            continue;
        }
        match bytes[i + 1] {
            code @ (b'P' | b'C') => {
                push(&mut out, &raw[start..i], primary);
                primary = code == b'P';
                start = i + 2;
                i += 2;
            }
            // `%%` keeps one `%`, which ends the run collected so far.
            b'%' => {
                push(&mut out, &raw[start..=i], primary);
                start = i + 2;
                i += 2;
            }
            _ => i += 1,
        }
    }
    push(&mut out, &raw[start..], primary);
    out
}

fn push<'a>(out: &mut Vec<Segment<'a>>, text: &'a str, primary: bool) {
    if !text.is_empty() {
        out.push(Segment { text, primary });
    }
}

#[cfg(test)]
#[path = "../../../tests/app/lang/markup.rs"]
mod tests;
