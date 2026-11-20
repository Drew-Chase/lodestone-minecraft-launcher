use super::*;
use crate::support::{put, scratch};

#[test]
fn a_file_hashes_to_its_sha1_in_lowercase_hex() {
    let root = scratch("hash-abc");
    let path = put(&root, "abc.txt", b"abc");
    assert_eq!(sha1_of_file(&path).expect("the file hashes"), "a9993e364706816aba3e25717850c26c9cd0d89d");
}

#[test]
fn hex_writes_two_digits_per_byte() {
    assert_eq!(hex(&[0x0a, 0xff, 0x00]), "0aff00");
}
