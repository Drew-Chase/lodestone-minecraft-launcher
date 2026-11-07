use super::*;

/// The fingerprint of `bytes`, taken in pieces of `piece` bytes. It must give the same answer as taking the file whole.
fn in_pieces(bytes: &[u8], piece: usize) -> u32 {
    let mut hasher = Fingerprint::new(kept_len(bytes));
    for chunk in bytes.chunks(piece) {
        hasher.update(chunk);
    }
    hasher.finish()
}

// The reference values below come from an independent C implementation of MurmurHash2 with seed 1, applied to the bytes the
// fingerprint keeps.

#[test]
fn an_empty_file_hashes_to_the_finished_seed() {
    assert_eq!(fingerprint(b""), 1_540_447_798);
}

#[test]
fn a_file_fingerprints_to_the_hash_of_the_bytes_it_keeps() {
    assert_eq!(fingerprint(b"a"), 626_045_324);
    assert_eq!(fingerprint(b"abc"), 1_621_425_345);
    assert_eq!(fingerprint(b"abcd"), 3_376_380_438);
    assert_eq!(fingerprint(b"abcde"), 3_469_237_630);
    assert_eq!(fingerprint(b"1234567"), 2_741_603_086, "one block and a tail of three");
    assert_eq!(fingerprint(b"12345678"), 2_481_897_245, "whole blocks and no tail");
    assert_eq!(fingerprint(b"lodestone-fingerprint-check-0123456789"), 3_660_018_063);
}

#[test]
fn the_whitespace_the_fingerprint_leaves_out_does_not_change_it() {
    assert_eq!(fingerprint(b" \t\r\n"), fingerprint(b""));
    assert_eq!(fingerprint(b"a b\tc\r\nd"), fingerprint(b"abcd"));
    assert_eq!(fingerprint(b"The quick brown fox jumps over the lazy dog"), 3_751_777_527);
}

#[test]
fn any_other_byte_counts_toward_the_fingerprint() {
    assert_ne!(fingerprint(b"a\x0cb"), fingerprint(b"ab"), "a form feed is kept");
    assert_ne!(fingerprint(b"a\x0bb"), fingerprint(b"ab"), "a vertical tab is kept");
}

#[test]
fn a_file_read_in_pieces_fingerprints_as_it_does_whole() {
    let bytes = b"The quick brown fox jumps over the lazy dog, twice: The quick brown fox jumps over the lazy dog.\r\n";
    for piece in [1, 2, 3, 4, 5, 7, 64] {
        assert_eq!(in_pieces(bytes, piece), fingerprint(bytes), "pieces of {piece} bytes");
    }
}

#[test]
fn the_kept_length_counts_only_the_bytes_the_fingerprint_keeps() {
    assert_eq!(kept_len(b"a b\tc\r\nd"), 4);
    assert_eq!(kept_len(b" \t\r\n"), 0);
    assert_eq!(kept_len(b""), 0);
}
