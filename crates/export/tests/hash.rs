use super::*;

#[test]
fn digests_match_the_published_test_vectors_for_abc() {
    let digests = digest_reader(&b"abc"[..]).expect("reads");
    assert_eq!(digests.sha1, "a9993e364706816aba3e25717850c26c9cd0d89d");
    assert_eq!(
        digests.sha512,
        "ddaf35a193617abacc417349ae20413112e6fa4e89a97ea20a9eeee64b55d39a2192992a274fc1a836ba3c23a3feebbd454d4423643ce80e2a9ac94fa54ca49f"
    );
}

#[test]
fn a_file_hashes_to_the_digests_of_its_bytes() {
    let dir = crate::support::scratch("hash-file");
    let path = crate::support::put(&dir, "mods/abc.jar", b"abc");
    assert_eq!(digest_file(&path).expect("reads"), digest_reader(&b"abc"[..]).expect("reads"));
}
