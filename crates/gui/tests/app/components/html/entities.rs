use super::*;

#[test]
fn text_without_references_is_borrowed() {
    assert!(matches!(decode("plain text"), Cow::Borrowed("plain text")));
}

#[test]
fn named_references_decode() {
    assert_eq!(decode("Ant&amp;Bee &lt;3 &rsquo;s &nbsp;x"), "Ant&Bee <3 ’s \u{A0}x");
    assert_eq!(decode("Pi&ntilde;ata&hellip;"), "Piñata…");
}

#[test]
fn decimal_and_hex_references_decode() {
    assert_eq!(decode("&#8226; &#x2022; &#X2022;"), "• • •");
}

#[test]
fn a_reference_to_nothing_is_the_replacement_character() {
    assert_eq!(decode("a&#0;b"), "a\u{FFFD}b");
}

#[test]
fn unknown_and_unterminated_references_stay_as_written() {
    assert_eq!(decode("&bogus; &amp &#xZZ; &#55296;"), "&bogus; &amp &#xZZ; &#55296;");
    assert_eq!(decode("Q&A"), "Q&A");
}

#[test]
fn a_far_semicolon_does_not_end_a_reference() {
    assert_eq!(decode("&notareference at all; yes"), "&notareference at all; yes");
}

#[test]
fn a_reference_next_to_multibyte_text_does_not_split_a_character() {
    assert_eq!(decode("&ééééééééééé;"), "&ééééééééééé;");
}
