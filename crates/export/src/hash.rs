//! The digests a package records for a file: SHA-1 and SHA-512, as lowercase hex. Modrinth packs need
//! both, and the platforms match a mod by its SHA-1.

use std::fs::File;
use std::io::{self, Read};
use std::path::Path;

use sha1::{Digest, Sha1};
use sha2::Sha512;

/// The SHA-1 and SHA-512 of one file, as lowercase hex.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Digests {
    pub sha1: String,
    pub sha512: String,
}

/// The digests of everything `reader` yields, read in one pass.
pub fn digest_reader(mut reader: impl Read) -> io::Result<Digests> {
    let mut sha1 = Sha1::new();
    let mut sha512 = Sha512::new();
    let mut buffer = vec![0u8; 64 * 1024];
    loop {
        let read = reader.read(&mut buffer)?;
        if read == 0 {
            break;
        }
        sha1.update(&buffer[..read]);
        sha512.update(&buffer[..read]);
    }
    Ok(Digests {
        sha1: format!("{:x}", sha1.finalize()),
        sha512: format!("{:x}", sha512.finalize()),
    })
}

/// The digests of the file at `path`.
pub fn digest_file(path: &Path) -> io::Result<Digests> {
    digest_reader(File::open(path)?)
}

#[cfg(test)]
#[path = "../tests/hash.rs"]
mod tests;
