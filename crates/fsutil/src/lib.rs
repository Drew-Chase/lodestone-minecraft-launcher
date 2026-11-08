//! Writing files so a crash or a power cut never leaves a half-written one, and copying archive entries with limits, so a
//! hostile archive cannot fill the disk.
//!
//! Every persistent file the launcher keeps (preferences, credentials, pid files, version JSON) goes through
//! [`write_atomic`] or [`write_private`]. Archive entries that the launcher unpacks go through [`copy_entry`], which holds
//! each one to the size its header declares.

use std::fs::{self, OpenOptions};
use std::io::{self, Read, Write};
use std::path::{Path, PathBuf};
use std::sync::Arc;
use std::sync::atomic::{AtomicU64, Ordering};
use std::time::{SystemTime, UNIX_EPOCH};

/// The most an entry may grow to, relative to its compressed size, before it is refused as a likely bomb.
const MAX_RATIO: u64 = 100;
/// Room every entry gets regardless of its ratio, so small files that compress well still unpack.
const RATIO_SLACK: u64 = 16 << 20;
/// The largest single entry an archive may declare. The biggest file a Minecraft install writes is a region file of about 1 GiB.
const MAX_ENTRY: u64 = 4 << 30;

/// Writes `bytes` to `path` so that `path` is either its old contents or the new ones, never a part of either. The bytes go to a
/// temporary file beside `path`, which is flushed to disk and then renamed over it. Missing parent folders are created.
pub fn write_atomic(path: &Path, bytes: &[u8]) -> io::Result<()> {
    write(path, bytes, false)
}

/// [`write_atomic`] for a file only its owner may read, such as one that holds sign-in tokens. On Unix the file is created
/// owner-only from the start, so no other user can read it, not even for a moment.
pub fn write_private(path: &Path, bytes: &[u8]) -> io::Result<()> {
    write(path, bytes, true)
}

fn write(path: &Path, bytes: &[u8], private: bool) -> io::Result<()> {
    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent)?;
    }
    let temp = temporary_beside(path);
    let result = (|| {
        let mut options = OpenOptions::new();
        options.write(true).create_new(true);
        restrict(&mut options, private);
        let mut file = options.open(&temp)?;
        file.write_all(bytes)?;
        file.sync_all()?;
        drop(file);
        fs::rename(&temp, path)
    })();
    if result.is_err() {
        let _ = fs::remove_file(&temp);
    }
    result
}

/// A name beside `path` that no other write is using: this process, and a count of the writes it has made.
fn temporary_beside(path: &Path) -> PathBuf {
    static NEXT: AtomicU64 = AtomicU64::new(0);
    let serial = NEXT.fetch_add(1, Ordering::Relaxed);
    let name = path.file_name().map_or_else(|| "file".into(), |name| name.to_string_lossy());
    path.with_file_name(format!(".{name}.{}-{serial}.tmp", std::process::id()))
}

#[cfg(unix)]
fn restrict(options: &mut OpenOptions, private: bool) {
    use std::os::unix::fs::OpenOptionsExt;
    if private {
        options.mode(0o600);
    }
}

#[cfg(not(unix))]
fn restrict(_options: &mut OpenOptions, _private: bool) {}

/// Copies one archive entry into `writer`, holding it to what its header declares. `declared` is the uncompressed size the
/// header names, and `compressed` the size it takes in the archive. An entry that claims far more than it was compressed from,
/// or more than [`MAX_ENTRY`], is refused before anything is written. An entry whose data runs past its declared size is
/// refused too, since a header that lies about its size would otherwise let it write without limit.
pub fn copy_entry(reader: &mut impl Read, writer: &mut impl Write, declared: u64, compressed: u64) -> io::Result<u64> {
    let limit = compressed.saturating_mul(MAX_RATIO).saturating_add(RATIO_SLACK);
    if declared > limit || declared > MAX_ENTRY {
        return Err(io::Error::other(format!(
            "an archive entry claims {declared} bytes from {compressed} compressed, which is refused"
        )));
    }
    let copied = io::copy(&mut reader.by_ref().take(declared.saturating_add(1)), writer)?;
    if copied > declared {
        return Err(io::Error::other("an archive entry holds more bytes than its header declares"));
    }
    Ok(copied)
}

/// Moves a file that could not be read aside, to a name beside it that carries the time it was moved. The file is kept for
/// recovery, and its path is free for a fresh one. Returns the new path.
pub fn quarantine(path: &Path) -> io::Result<PathBuf> {
    let stamp = SystemTime::now().duration_since(UNIX_EPOCH).unwrap_or_default().as_secs();
    let name = path.file_name().map_or_else(|| "file".into(), |name| name.to_string_lossy());
    let target = path.with_file_name(format!("{name}.corrupt-{stamp}"));
    fs::rename(path, &target)?;
    Ok(target)
}

/// A reader that counts the bytes read through it. The count is shared with the caller, so it can be read while the reader is in use.
pub struct Counted<R> {
    inner: R,
    count: Arc<AtomicU64>,
}

impl<R: Read> Counted<R> {
    /// Wraps `inner`, and returns the reader with the count it keeps.
    pub fn new(inner: R) -> (Self, Arc<AtomicU64>) {
        let count = Arc::new(AtomicU64::new(0));
        (
            Self {
                inner,
                count: Arc::clone(&count),
            },
            count,
        )
    }
}

impl<R: Read> Read for Counted<R> {
    fn read(&mut self, buf: &mut [u8]) -> io::Result<usize> {
        let read = self.inner.read(buf)?;
        self.count.fetch_add(read as u64, Ordering::Relaxed);
        Ok(read)
    }
}

/// A reader of decompressed data that refuses to go far past the compressed bytes behind it. `consumed` is the count of the
/// compressed input (see [`Counted`]). Once the output is more than [`MAX_RATIO`] times that, plus [`RATIO_SLACK`], the read
/// fails, so a stream that expands far beyond its size stops rather than filling the disk.
pub struct Bounded<R> {
    inner: R,
    consumed: Arc<AtomicU64>,
    produced: u64,
}

impl<R: Read> Bounded<R> {
    /// Wraps `inner`, the decompressed output, whose compressed input `consumed` counts.
    pub fn new(inner: R, consumed: Arc<AtomicU64>) -> Self {
        Self {
            inner,
            consumed,
            produced: 0,
        }
    }
}

impl<R: Read> Read for Bounded<R> {
    fn read(&mut self, buf: &mut [u8]) -> io::Result<usize> {
        let read = self.inner.read(buf)?;
        self.produced = self.produced.saturating_add(read as u64);
        let allowed = self
            .consumed
            .load(Ordering::Relaxed)
            .saturating_mul(MAX_RATIO)
            .saturating_add(RATIO_SLACK);
        if self.produced > allowed {
            return Err(io::Error::other(
                "the decompressed data is far larger than the compressed data it came from, so it is refused",
            ));
        }
        Ok(read)
    }
}

/// Whether an archive whose entries declare `declared_total` bytes between them is plausible for a file of `archive_len` bytes.
/// Checked before any entry is written, so a bomb with many entries is refused as a whole.
pub fn archive_fits(declared_total: u64, archive_len: u64) -> bool {
    declared_total <= archive_len.saturating_mul(MAX_RATIO).saturating_add(RATIO_SLACK)
}

#[cfg(test)]
#[path = "../tests/lib.rs"]
mod tests;
