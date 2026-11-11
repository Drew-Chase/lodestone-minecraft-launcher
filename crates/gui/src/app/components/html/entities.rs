//! Character references: the numeric ones, and the named ones that platform editors write. A reference this does not
//! know stays as it was written, the way a browser shows a reference it cannot read.

use std::borrow::Cow;

/// The most bytes a reference takes, its `&` and `;` included. A `;` further on does not end a reference.
const MAX_LENGTH: usize = 12;

/// The text with its character references decoded. Text without an `&` is returned as it is.
pub fn decode(raw: &str) -> Cow<'_, str> {
    if !raw.contains('&') {
        return Cow::Borrowed(raw);
    }
    let mut out = String::with_capacity(raw.len());
    let mut rest = raw;
    while let Some(at) = rest.find('&') {
        out.push_str(&rest[..at]);
        let tail = &rest[at..];
        match reference(tail) {
            Some((character, length)) => {
                out.push(character);
                rest = &tail[length..];
            }
            None => {
                out.push('&');
                rest = &tail[1..];
            }
        }
    }
    out.push_str(rest);
    Cow::Owned(out)
}

/// The character a reference at the start of `text` stands for, and the bytes the reference takes. The reference
/// must end in `;`. A numeric reference to nothing (`&#0;`) stands for the replacement character.
fn reference(text: &str) -> Option<(char, usize)> {
    let end = text.bytes().take(MAX_LENGTH).position(|byte| byte == b';')?;
    let name = &text[1..end];
    let character = if let Some(hex) = name.strip_prefix("#x").or_else(|| name.strip_prefix("#X")) {
        numeric(u32::from_str_radix(hex, 16).ok()?)?
    } else if let Some(decimal) = name.strip_prefix('#') {
        numeric(decimal.parse().ok()?)?
    } else {
        named(name)?
    };
    Some((character, end + 1))
}

/// The character a numeric reference names. Zero names the replacement character, and a surrogate names nothing.
fn numeric(value: u32) -> Option<char> {
    if value == 0 { Some('\u{FFFD}') } else { char::from_u32(value) }
}

/// The character a named reference stands for, among the ones platform editors write.
fn named(name: &str) -> Option<char> {
    Some(match name {
        "amp" => '&',
        "lt" => '<',
        "gt" => '>',
        "quot" => '"',
        "apos" => '\'',
        "nbsp" => '\u{A0}',
        "ensp" => '\u{2002}',
        "emsp" => '\u{2003}',
        "thinsp" => '\u{2009}',
        "zwnj" => '\u{200C}',
        "zwj" => '\u{200D}',
        "shy" => '\u{AD}',
        "ndash" => '–',
        "mdash" => '—',
        "lsquo" => '‘',
        "rsquo" => '’',
        "sbquo" => '‚',
        "ldquo" => '“',
        "rdquo" => '”',
        "bdquo" => '„',
        "laquo" => '«',
        "raquo" => '»',
        "lsaquo" => '‹',
        "rsaquo" => '›',
        "hellip" => '…',
        "bull" => '•',
        "middot" => '·',
        "prime" => '′',
        "Prime" => '″',
        "dagger" => '†',
        "Dagger" => '‡',
        "permil" => '‰',
        "copy" => '©',
        "reg" => '®',
        "trade" => '™',
        "deg" => '°',
        "plusmn" => '±',
        "times" => '×',
        "divide" => '÷',
        "micro" => 'µ',
        "para" => '¶',
        "sect" => '§',
        "cent" => '¢',
        "pound" => '£',
        "euro" => '€',
        "yen" => '¥',
        "curren" => '¤',
        "frac12" => '½',
        "frac14" => '¼',
        "frac34" => '¾',
        "sup1" => '¹',
        "sup2" => '²',
        "sup3" => '³',
        "ordf" => 'ª',
        "ordm" => 'º',
        "iexcl" => '¡',
        "iquest" => '¿',
        "not" => '¬',
        "larr" => '←',
        "rarr" => '→',
        "uarr" => '↑',
        "darr" => '↓',
        "harr" => '↔',
        "lArr" => '⇐',
        "rArr" => '⇒',
        "hearts" => '♥',
        "spades" => '♠',
        "clubs" => '♣',
        "diams" => '♦',
        "check" => '✓',
        "cross" => '✗',
        "star" => '☆',
        "starf" => '★',
        "Agrave" => 'À',
        "Aacute" => 'Á',
        "Acirc" => 'Â',
        "Atilde" => 'Ã',
        "Auml" => 'Ä',
        "Aring" => 'Å',
        "AElig" => 'Æ',
        "Ccedil" => 'Ç',
        "Egrave" => 'È',
        "Eacute" => 'É',
        "Ecirc" => 'Ê',
        "Euml" => 'Ë',
        "Igrave" => 'Ì',
        "Iacute" => 'Í',
        "Icirc" => 'Î',
        "Iuml" => 'Ï',
        "ETH" => 'Ð',
        "Ntilde" => 'Ñ',
        "Ograve" => 'Ò',
        "Oacute" => 'Ó',
        "Ocirc" => 'Ô',
        "Otilde" => 'Õ',
        "Ouml" => 'Ö',
        "Oslash" => 'Ø',
        "Ugrave" => 'Ù',
        "Uacute" => 'Ú',
        "Ucirc" => 'Û',
        "Uuml" => 'Ü',
        "Yacute" => 'Ý',
        "THORN" => 'Þ',
        "szlig" => 'ß',
        "agrave" => 'à',
        "aacute" => 'á',
        "acirc" => 'â',
        "atilde" => 'ã',
        "auml" => 'ä',
        "aring" => 'å',
        "aelig" => 'æ',
        "ccedil" => 'ç',
        "egrave" => 'è',
        "eacute" => 'é',
        "ecirc" => 'ê',
        "euml" => 'ë',
        "igrave" => 'ì',
        "iacute" => 'í',
        "icirc" => 'î',
        "iuml" => 'ï',
        "eth" => 'ð',
        "ntilde" => 'ñ',
        "ograve" => 'ò',
        "oacute" => 'ó',
        "ocirc" => 'ô',
        "otilde" => 'õ',
        "ouml" => 'ö',
        "oslash" => 'ø',
        "ugrave" => 'ù',
        "uacute" => 'ú',
        "ucirc" => 'û',
        "uuml" => 'ü',
        "yacute" => 'ý',
        "thorn" => 'þ',
        "yuml" => 'ÿ',
        _ => return None,
    })
}

#[cfg(test)]
#[path = "../../../../tests/app/components/html/entities.rs"]
mod tests;
