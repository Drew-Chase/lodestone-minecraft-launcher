use super::*;

const HASH: &str = "ba7816bf8f01cfea414140de5dae2223b00361a396177a9cb410ff61f20015ad";
const SETUP: &str = "Lodestone-v0.2.0-setup.exe";

#[test]
fn finds_the_named_file_among_others() {
    let sums = format!("{}  Lodestone-v0.2.0-linux-x86_64.tar.gz\n{HASH}  {SETUP}\n", "0".repeat(64));
    assert_eq!(checksum_for(&sums, SETUP).as_deref(), Some(HASH));
    assert!(checksum_for(&sums, "missing.exe").is_none());
}

#[test]
fn accepts_binary_mode_crlf_and_upper_case_hex() {
    let sums = format!("{}  *{SETUP}\r\n", HASH.to_uppercase());
    assert_eq!(checksum_for(&sums, SETUP).as_deref(), Some(HASH));
}

#[test]
fn ignores_lines_that_are_not_a_sha256() {
    assert!(checksum_for(&format!("abc  {SETUP}\n"), SETUP).is_none());
    assert!(checksum_for(&format!("{HASH}0  {SETUP}\n"), SETUP).is_none());
}
