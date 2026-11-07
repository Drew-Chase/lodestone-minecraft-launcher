//! CurseForge's file fingerprint. CurseForge names a file by the MurmurHash2 (32-bit, seed 1) of its bytes, with the tab,
//! line feed, carriage return and space bytes left out. A file can therefore be recognised on CurseForge from its bytes alone.
//!
//! A file is hashed as it is read. The hash starts from the number of bytes it keeps, so the caller counts those first with
//! [`kept_len`] and then feeds the bytes to a [`Fingerprint`]. A large file never has to be held in memory.

/// MurmurHash2's multiplier.
const M: u32 = 0x5bd1_e995;
/// The bits each block is shifted down before it is mixed.
const R: u32 = 24;
/// The seed CurseForge's fingerprint uses.
const SEED: u32 = 1;

/// Whether a byte is one the fingerprint leaves out: a tab, a line feed, a carriage return or a space.
fn is_left_out(byte: u8) -> bool {
    matches!(byte, b'\t' | b'\n' | b'\r' | b' ')
}

/// How many of `bytes` a fingerprint keeps. A fingerprint starts from the count over the whole file, so it is taken before the
/// file is hashed.
pub fn kept_len(bytes: &[u8]) -> u64 {
    bytes.iter().filter(|byte| !is_left_out(**byte)).count() as u64
}

/// The fingerprint of a file's bytes, taken all at once.
pub fn fingerprint(bytes: &[u8]) -> u32 {
    let mut hasher = Fingerprint::new(kept_len(bytes));
    hasher.update(bytes);
    hasher.finish()
}

/// A fingerprint taken as a file is read, in pieces. It starts from the `kept_length` of the whole file, which is what
/// [`kept_len`] gives over every byte of it, or the hash is wrong.
#[derive(Debug, Clone)]
pub struct Fingerprint {
    /// The running hash.
    hash: u32,
    /// The kept bytes that have not yet filled a block.
    block: [u8; 4],
    /// How many bytes of `block` are filled.
    filled: usize,
}

impl Fingerprint {
    /// A fingerprint of a file that keeps `kept_length` bytes.
    pub fn new(kept_length: u64) -> Self {
        Self {
            hash: SEED ^ (kept_length as u32),
            block: [0; 4],
            filled: 0,
        }
    }

    /// Takes the next bytes of the file. The bytes the fingerprint leaves out are skipped.
    pub fn update(&mut self, bytes: &[u8]) {
        for &byte in bytes {
            if is_left_out(byte) {
                continue;
            }
            self.block[self.filled] = byte;
            self.filled += 1;
            if self.filled == self.block.len() {
                self.mix_block();
                self.filled = 0;
            }
        }
    }

    /// Folds the full block into the hash.
    fn mix_block(&mut self) {
        let mut block = u32::from_le_bytes(self.block).wrapping_mul(M);
        block ^= block >> R;
        block = block.wrapping_mul(M);
        self.hash = self.hash.wrapping_mul(M) ^ block;
    }

    /// The fingerprint of the file, once every one of its bytes is taken.
    pub fn finish(mut self) -> u32 {
        if self.filled > 0 {
            for index in (0..self.filled).rev() {
                self.hash ^= u32::from(self.block[index]) << (8 * index);
            }
            self.hash = self.hash.wrapping_mul(M);
        }
        let mut hash = self.hash;
        hash ^= hash >> 13;
        hash = hash.wrapping_mul(M);
        hash ^ (hash >> 15)
    }
}

#[cfg(test)]
#[path = "../tests/fingerprint.rs"]
mod tests;
