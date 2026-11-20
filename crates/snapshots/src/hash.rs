//! SHA-1 digests, as snapshots record a jar's bytes.

use std::fmt::Write;
use std::fs::File;
use std::io::{self, Read};
use std::path::Path;

use sha1::{Digest, Sha1};

/// The bytes a digest reads at a time.
const CHUNK: usize = 64 * 1024;

/// The SHA-1 of the file at `path`, as lowercase hex.
pub fn sha1_of_file(path: &Path) -> io::Result<String> {
    let mut file = File::open(path)?;
    let mut hasher = Sha1::new();
    let mut buffer = vec![0; CHUNK];
    loop {
        let read = file.read(&mut buffer)?;
        if read == 0 {
            break;
        }
        hasher.update(&buffer[..read]);
    }
    Ok(hex(&hasher.finalize()))
}

/// `bytes` as lowercase hex.
pub fn hex(bytes: &[u8]) -> String {
    bytes.iter().fold(String::with_capacity(bytes.len() * 2), |mut out, byte| {
        let _ = write!(out, "{byte:02x}");
        out
    })
}

#[cfg(test)]
#[path = "../tests/hash.rs"]
mod tests;
