use base64::engine::general_purpose::STANDARD;
use ed25519_dalek::{Signer, SigningKey};

use super::*;

/// The key id every test key carries. minisign names a key by it.
const KEY_ID: &[u8; 8] = b"lodestne";

/// The checksum list the tests sign.
const SUMS: &[u8] = b"abc123  file.jar\n";

fn key(seed: u8) -> SigningKey {
    SigningKey::from_bytes(&[seed; 32])
}

/// minisign's public key text for `key`.
fn public_text(key: &SigningKey) -> String {
    let mut bytes = b"Ed".to_vec();
    bytes.extend_from_slice(KEY_ID);
    bytes.extend_from_slice(key.verifying_key().as_bytes());
    format!("untrusted comment: test key\n{}\n", STANDARD.encode(bytes))
}

/// minisign's signature file for `sums` made by `key`, with `comment` as its trusted comment.
fn signature_text(key: &SigningKey, sums: &[u8], comment: &str) -> String {
    signature_with_id(key, KEY_ID, sums, comment)
}

/// [`signature_text`] naming the key `id`.
fn signature_with_id(key: &SigningKey, id: &[u8; 8], sums: &[u8], comment: &str) -> String {
    let signature = key.sign(sums);
    let mut bytes = b"Ed".to_vec();
    bytes.extend_from_slice(id);
    bytes.extend_from_slice(&signature.to_bytes());
    let mut global = signature.to_bytes().to_vec();
    global.extend_from_slice(comment.as_bytes());
    let global = key.sign(&global);
    format!(
        "untrusted comment: test\n{}\ntrusted comment: {comment}\n{}\n",
        STANDARD.encode(bytes),
        STANDARD.encode(global.to_bytes())
    )
}

#[test]
fn a_list_signed_by_the_release_key_is_trusted() {
    let key = key(7);
    let signature = signature_text(&key, SUMS, "release 1");
    assert_eq!(check_with_key(&public_text(&key), SUMS, Some(&signature)).unwrap(), Trust::Verified);
}

#[test]
fn with_no_key_configured_the_checksums_stand_alone() {
    assert_eq!(check_with_key("  \n", SUMS, None).unwrap(), Trust::Unconfigured);
}

#[test]
fn a_release_without_a_signature_is_refused_once_a_key_is_configured() {
    let key = key(7);
    let error = check_with_key(&public_text(&key), SUMS, None).unwrap_err();
    assert!(matches!(error, Error::Signature(_)), "{error}");
}

#[test]
fn a_list_changed_after_it_was_signed_is_refused() {
    let key = key(7);
    let signature = signature_text(&key, SUMS, "release 1");
    let error = check_with_key(&public_text(&key), b"evil999  file.jar\n", Some(&signature)).unwrap_err();
    assert!(error.to_string().contains("do not match"), "{error}");
}

#[test]
fn a_list_signed_by_another_key_is_refused() {
    let trusted = key(7);
    let other = key(9);
    let signature = signature_text(&other, SUMS, "release 1");
    assert!(check_with_key(&public_text(&trusted), SUMS, Some(&signature)).is_err());
}

#[test]
fn a_signature_naming_another_key_id_is_refused_by_name() {
    let key = key(7);
    let signature = signature_with_id(&key, b"otherkey", SUMS, "release 1");
    let error = check_with_key(&public_text(&key), SUMS, Some(&signature)).unwrap_err();
    assert!(error.to_string().contains("different key"), "{error}");
}

#[test]
fn a_changed_trusted_comment_is_refused_by_the_global_signature() {
    let key = key(7);
    let signature = signature_text(&key, SUMS, "release 1").replace("release 1", "release 2");
    let error = check_with_key(&public_text(&key), SUMS, Some(&signature)).unwrap_err();
    assert!(error.to_string().contains("trusted comment"), "{error}");
}

#[test]
fn a_malformed_signature_file_is_refused() {
    let key = key(7);
    assert!(check_with_key(&public_text(&key), SUMS, Some("garbage")).is_err());
    assert!(check_with_key("not a key", SUMS, Some("")).is_err());
}
