//! Reading a file's bytes as text, and writing the text back the way the file was.
//!
//! Minecraft's files are mostly UTF-8, but a config edited on Windows may be UTF-16 with a byte order mark, and an old
//! one may be in the Western European code page that Windows calls ANSI. The encoding is found from the bytes, kept
//! with the text, and used again on save, so a file is never silently changed to another encoding. Bytes that are no
//! kind of text are refused, so a jar or a world file never opens as garbage.

/// How a file's text is stored.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Encoding {
    Utf8,
    /// UTF-8 with a byte order mark in front.
    Utf8Bom,
    /// UTF-16, low byte first, with its byte order mark.
    Utf16Le,
    /// UTF-16, high byte first, with its byte order mark.
    Utf16Be,
    /// Windows-1252, the "ANSI" code page of Western European Windows.
    Windows1252,
}

impl Encoding {
    /// The name the editor's footer shows.
    pub fn label(self) -> &'static str {
        match self {
            Self::Utf8 => "UTF-8",
            Self::Utf8Bom => "UTF-8 BOM",
            Self::Utf16Le => "UTF-16 LE",
            Self::Utf16Be => "UTF-16 BE",
            Self::Windows1252 => "ANSI",
        }
    }
}

/// The characters Windows-1252 puts at 0x80 to 0x9F, where Latin-1 has control codes. The five bytes it leaves
/// undefined decode as the control code of the same number, as browsers decode them.
const HIGH: [char; 32] = [
    '\u{20AC}', '\u{81}', '\u{201A}', '\u{192}', '\u{201E}', '\u{2026}', '\u{2020}', '\u{2021}', '\u{2C6}', '\u{2030}', '\u{160}', '\u{2039}',
    '\u{152}', '\u{8D}', '\u{17D}', '\u{8F}', '\u{90}', '\u{2018}', '\u{2019}', '\u{201C}', '\u{201D}', '\u{2022}', '\u{2013}', '\u{2014}',
    '\u{2DC}', '\u{2122}', '\u{161}', '\u{203A}', '\u{153}', '\u{9D}', '\u{17E}', '\u{178}',
];

/// The text of `bytes` and the encoding it was read in. `None` when the bytes are not text.
pub fn decode(bytes: &[u8]) -> Option<(String, Encoding)> {
    if let Some(rest) = bytes.strip_prefix(&[0xEF, 0xBB, 0xBF]) {
        return text_only(String::from_utf8(rest.to_vec()).ok()?, Encoding::Utf8Bom);
    }
    if let Some(rest) = bytes.strip_prefix(&[0xFF, 0xFE]) {
        return text_only(utf16(rest, u16::from_le_bytes)?, Encoding::Utf16Le);
    }
    if let Some(rest) = bytes.strip_prefix(&[0xFE, 0xFF]) {
        return text_only(utf16(rest, u16::from_be_bytes)?, Encoding::Utf16Be);
    }
    match std::str::from_utf8(bytes) {
        Ok(text) => text_only(text.to_string(), Encoding::Utf8),
        // Not UTF-8. A file with no zero byte and no stray control codes is taken as Windows-1252, which gives every
        // byte a character.
        Err(_) => text_only(bytes.iter().map(|&byte| from_1252(byte)).collect(), Encoding::Windows1252),
    }
}

/// `text` with its encoding, unless it holds what no text file does: a zero byte, or control codes other than the
/// tab, the line endings, the form feed and the escape that starts a colour code in a log.
fn text_only(text: String, encoding: Encoding) -> Option<(String, Encoding)> {
    let binary = text
        .chars()
        .any(|character| character.is_control() && !matches!(character, '\t' | '\n' | '\r' | '\u{C}' | '\u{1B}') && (character as u32) < 0x80);
    (!binary).then_some((text, encoding))
}

/// The text of UTF-16 `bytes`, whose pairs `unit` joins. `None` for an odd length or a broken surrogate pair.
fn utf16(bytes: &[u8], unit: fn([u8; 2]) -> u16) -> Option<String> {
    if !bytes.len().is_multiple_of(2) {
        return None;
    }
    let units = bytes.chunks_exact(2).map(|pair| unit([pair[0], pair[1]]));
    char::decode_utf16(units).collect::<Result<String, _>>().ok()
}

/// The character Windows-1252 gives `byte`.
fn from_1252(byte: u8) -> char {
    match byte {
        0x80..=0x9F => HIGH[usize::from(byte - 0x80)],
        _ => char::from(byte),
    }
}

/// The byte Windows-1252 gives `character`, when it has one.
fn to_1252(character: char) -> Option<u8> {
    let code = character as u32;
    if code < 0x80 || (0xA0..=0xFF).contains(&code) {
        return u8::try_from(code).ok();
    }
    HIGH.iter()
        .position(|high| *high == character)
        .and_then(|index| u8::try_from(0x80 + index).ok())
}

/// The bytes of `text` in `encoding`, with the byte order mark the encoding carries. Text that Windows-1252 cannot hold
/// comes back as UTF-8 instead, and the encoding returned says which one the bytes are in.
pub fn encode(text: &str, encoding: Encoding) -> (Vec<u8>, Encoding) {
    match encoding {
        Encoding::Utf8 => (text.as_bytes().to_vec(), encoding),
        Encoding::Utf8Bom => {
            let mut bytes = vec![0xEF, 0xBB, 0xBF];
            bytes.extend_from_slice(text.as_bytes());
            (bytes, encoding)
        }
        Encoding::Utf16Le => (utf16_bytes(text, [0xFF, 0xFE], u16::to_le_bytes), encoding),
        Encoding::Utf16Be => (utf16_bytes(text, [0xFE, 0xFF], u16::to_be_bytes), encoding),
        Encoding::Windows1252 => match text.chars().map(to_1252).collect::<Option<Vec<u8>>>() {
            Some(bytes) => (bytes, encoding),
            None => (text.as_bytes().to_vec(), Encoding::Utf8),
        },
    }
}

/// `text` as UTF-16 after the byte order mark `mark`, each unit split into bytes by `split`.
fn utf16_bytes(text: &str, mark: [u8; 2], split: fn(u16) -> [u8; 2]) -> Vec<u8> {
    let mut bytes = Vec::with_capacity(2 + text.len() * 2);
    bytes.extend_from_slice(&mark);
    for unit in text.encode_utf16() {
        bytes.extend_from_slice(&split(unit));
    }
    bytes
}

#[cfg(test)]
#[path = "../../../../tests/app/components/editor/encoding.rs"]
mod tests;
