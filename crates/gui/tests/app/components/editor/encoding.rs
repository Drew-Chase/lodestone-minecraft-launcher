use super::*;

/// Decodes `bytes`, and checks that encoding the text again gives the same bytes back.
fn round_trip(bytes: &[u8]) -> (String, Encoding) {
    let (text, encoding) = decode(bytes).expect("the bytes are text");
    assert_eq!(
        encode(&text, encoding),
        (bytes.to_vec(), encoding),
        "a file saved unchanged keeps its bytes"
    );
    (text, encoding)
}

#[test]
fn plain_utf8_is_read_as_utf8() {
    assert_eq!(
        round_trip("fov = 70\nname = \"café ✓\"\n".as_bytes()),
        ("fov = 70\nname = \"café ✓\"\n".to_string(), Encoding::Utf8)
    );
    assert_eq!(round_trip(b""), (String::new(), Encoding::Utf8), "an empty file is text");
}

#[test]
fn a_utf8_byte_order_mark_is_kept_out_of_the_text_and_written_back() {
    let (text, encoding) = round_trip(b"\xEF\xBB\xBF{\"a\": 1}");
    assert_eq!((text.as_str(), encoding), ("{\"a\": 1}", Encoding::Utf8Bom));
}

#[test]
fn utf16_is_read_in_either_byte_order() {
    let little = [0xFF, 0xFE, b'h', 0, b'i', 0, 0x3D, 0xD8, 0x00, 0xDE];
    assert_eq!(round_trip(&little), ("hi\u{1F600}".to_string(), Encoding::Utf16Le));
    let big = [0xFE, 0xFF, 0, b'h', 0, b'i', 0x00, 0xE9];
    assert_eq!(round_trip(&big), ("hi\u{E9}".to_string(), Encoding::Utf16Be));
}

#[test]
fn bytes_that_are_not_utf8_are_read_as_windows_1252() {
    // "café – ok" as Windows writes it: 0xE9 for é, and 0x96 for the dash.
    let bytes = b"caf\xE9 \x96 ok\r\n";
    assert_eq!(round_trip(bytes), ("caf\u{E9} \u{2013} ok\r\n".to_string(), Encoding::Windows1252));
}

#[test]
fn text_windows_1252_cannot_hold_is_saved_as_utf8() {
    let (bytes, encoding) = encode("price: 5 \u{20AC}", Encoding::Windows1252);
    assert_eq!(
        (bytes.as_slice(), encoding),
        (b"price: 5 \x80".as_slice(), Encoding::Windows1252),
        "the euro sign has a byte"
    );
    let (bytes, encoding) = encode("snow \u{2603}", Encoding::Windows1252);
    assert_eq!(encoding, Encoding::Utf8, "the snowman has none");
    assert_eq!(bytes, "snow \u{2603}".as_bytes());
}

#[test]
fn binary_files_are_refused() {
    assert_eq!(decode(b"PK\x03\x04\x14\x00\x00\x00"), None, "a jar is not text");
    assert_eq!(decode(b"\x0A\x00\x00\x08\x00\x04name"), None, "nor is NBT");
    assert_eq!(decode(&[0xFF, 0xFE, b'a']), None, "UTF-16 has an even number of bytes");
    assert_eq!(decode(&[0xFF, 0xFE, 0x00, 0xD8]), None, "a surrogate with no partner is not text");
}

#[test]
fn a_log_with_colour_codes_and_form_feeds_is_still_text() {
    let (text, encoding) = round_trip(b"\x1B[32mdone\x1B[0m\x0C\n\tindented\n");
    assert_eq!(encoding, Encoding::Utf8);
    assert!(text.contains("done"));
}

#[test]
fn each_encoding_has_a_label_for_the_footer() {
    let labels = [
        Encoding::Utf8,
        Encoding::Utf8Bom,
        Encoding::Utf16Le,
        Encoding::Utf16Be,
        Encoding::Windows1252,
    ]
    .map(Encoding::label);
    assert_eq!(labels, ["UTF-8", "UTF-8 BOM", "UTF-16 LE", "UTF-16 BE", "ANSI"]);
}
