//! Checks the release's `SHA256SUMS.txt` against the signature the release publishes beside it, in minisign's format.
//!
//! The checksum list is what every downloaded file is checked against, so it must come from the maintainers. A list that
//! anyone who can publish a release could write names whatever files they built. The signature proves the list came from
//! the holder of the release key, whose public half is in `release.pub` and is built into the launcher.
//!
//! While `release.pub` holds no key, no signature is required and the list is trusted as before. Once it holds one, a
//! release without a valid signature is refused.

use base64::Engine;
use base64::engine::general_purpose::STANDARD;
use ed25519_dalek::{Signature, Verifier, VerifyingKey};

use crate::error::Error;

/// The public key the launcher trusts, in minisign's two-line format. Empty until the release key is written here.
const RELEASE_PUBLIC_KEY: &str = include_str!("../release.pub");

/// The signature a release publishes beside its checksum list.
pub const SIGNATURE_FILE: &str = "SHA256SUMS.txt.minisig";

/// The algorithm tag minisign writes for an Ed25519 signature over the file itself.
const ALGORITHM: &[u8; 2] = b"Ed";

/// The outcome of checking a checksum list.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Trust {
    /// No release key is configured, so the list was not checked against a signature.
    Unconfigured,
    /// The list is signed by the configured release key.
    Verified,
}

/// Whether a release key is configured. A launcher without one checks downloads by their checksums alone.
pub fn configured() -> bool {
    !RELEASE_PUBLIC_KEY.trim().is_empty()
}

/// Checks `sums`, the bytes of the checksum list, against `signature`, the text of the release's signature file.
pub fn check_sums(sums: &[u8], signature: Option<&str>) -> Result<Trust, Error> {
    check_with_key(RELEASE_PUBLIC_KEY, sums, signature)
}

/// [`check_sums`] against the key in `public`, which is in minisign's format. An empty key means none is configured.
pub(crate) fn check_with_key(public: &str, sums: &[u8], signature: Option<&str>) -> Result<Trust, Error> {
    if public.trim().is_empty() {
        return Ok(Trust::Unconfigured);
    }
    let (key_id, key) = parse_public_key(public)?;
    let text = signature.ok_or_else(|| Error::Signature("the release has no signature for its checksums".to_string()))?;
    let parsed = parse_signature(text)?;
    if parsed.key_id != key_id {
        return Err(Error::Signature("the checksums are signed with a different key".to_string()));
    }
    key.verify(sums, &parsed.signature)
        .map_err(|_| Error::Signature("the checksums do not match their signature".to_string()))?;
    // The global signature covers the trusted comment too, so the comment cannot be changed without the key.
    let mut global = Vec::with_capacity(64 + parsed.trusted_comment.len());
    global.extend_from_slice(&parsed.signature.to_bytes());
    global.extend_from_slice(parsed.trusted_comment.as_bytes());
    key.verify(&global, &parsed.global)
        .map_err(|_| Error::Signature("the signature's trusted comment does not match".to_string()))?;
    Ok(Trust::Verified)
}

/// The key id and the verifying key of minisign's public key text.
fn parse_public_key(text: &str) -> Result<([u8; 8], VerifyingKey), Error> {
    let line = text
        .lines()
        .map(str::trim)
        .find(|line| !line.is_empty() && !line.starts_with("untrusted comment:"))
        .ok_or_else(|| Error::Signature("the release key is empty".to_string()))?;
    let bytes = decode(line, 42).map_err(|_| Error::Signature("the release key is not a minisign public key".to_string()))?;
    if &bytes[..2] != ALGORITHM {
        return Err(Error::Signature("the release key is not an Ed25519 key".to_string()));
    }
    let key_id: [u8; 8] = bytes[2..10]
        .try_into()
        .map_err(|_| Error::Signature("the release key is malformed".to_string()))?;
    let public: [u8; 32] = bytes[10..42]
        .try_into()
        .map_err(|_| Error::Signature("the release key is malformed".to_string()))?;
    let key = VerifyingKey::from_bytes(&public).map_err(|_| Error::Signature("the release key is not a valid Ed25519 key".to_string()))?;
    Ok((key_id, key))
}

/// A parsed signature file.
struct Parsed {
    key_id: [u8; 8],
    signature: Signature,
    trusted_comment: String,
    global: Signature,
}

/// Reads minisign's four-line signature file: an untrusted comment, the signature, the trusted comment and the global signature.
fn parse_signature(text: &str) -> Result<Parsed, Error> {
    let malformed = || Error::Signature("the release signature file is malformed".to_string());
    let lines: Vec<&str> = text.lines().map(str::trim).collect();
    let [_, signature_line, comment_line, global_line, ..] = lines.as_slice() else {
        return Err(malformed());
    };
    let trusted_comment = comment_line.strip_prefix("trusted comment: ").ok_or_else(malformed)?.to_string();
    let bytes = decode(signature_line, 74).map_err(|_| malformed())?;
    if &bytes[..2] != ALGORITHM {
        return Err(Error::Signature("the release signature is not an Ed25519 signature".to_string()));
    }
    let key_id: [u8; 8] = bytes[2..10].try_into().map_err(|_| malformed())?;
    let signature = Signature::from_slice(&bytes[10..74]).map_err(|_| malformed())?;
    let global_bytes = decode(global_line, 64).map_err(|_| malformed())?;
    let global = Signature::from_slice(&global_bytes).map_err(|_| malformed())?;
    Ok(Parsed {
        key_id,
        signature,
        trusted_comment,
        global,
    })
}

/// Decodes base64 `text` that must be exactly `len` bytes long.
fn decode(text: &str, len: usize) -> Result<Vec<u8>, base64::DecodeError> {
    let bytes = STANDARD.decode(text)?;
    if bytes.len() == len {
        Ok(bytes)
    } else {
        Err(base64::DecodeError::InvalidLength(bytes.len()))
    }
}

#[cfg(test)]
#[path = "../tests/signature.rs"]
mod tests;
