use std::io::{self, Cursor, Read};
use std::sync::Arc;
use std::sync::atomic::AtomicU64;

use super::*;

fn scratch(name: &str) -> PathBuf {
    let dir = std::env::temp_dir().join(format!("lodestone-fs-{name}-{}", std::process::id()));
    let _ = fs::remove_dir_all(&dir);
    dir
}

#[test]
fn an_atomic_write_replaces_the_file_and_leaves_no_temporary_behind() {
    let dir = scratch("atomic");
    let path = dir.join("nested").join("prefs.json");
    write_atomic(&path, b"one").unwrap();
    write_atomic(&path, b"two").unwrap();
    assert_eq!(fs::read(&path).unwrap(), b"two");
    let names: Vec<_> = fs::read_dir(path.parent().unwrap())
        .unwrap()
        .flatten()
        .map(|entry| entry.file_name())
        .collect();
    assert_eq!(names.len(), 1, "only the file itself is left: {names:?}");
}

#[cfg(unix)]
#[test]
fn a_private_write_is_owner_only_from_the_start() {
    use std::os::unix::fs::PermissionsExt;
    let dir = scratch("private");
    let path = dir.join("credentials.json");
    write_private(&path, b"{}").unwrap();
    assert_eq!(fs::metadata(&path).unwrap().permissions().mode() & 0o777, 0o600);
}

#[test]
fn an_entry_copies_when_it_matches_its_header() {
    let data = vec![7u8; 1000];
    let mut out = Vec::new();
    let copied = copy_entry(&mut Cursor::new(&data), &mut out, 1000, 500).unwrap();
    assert_eq!(copied, 1000);
    assert_eq!(out, data);
}

#[test]
fn an_entry_that_runs_past_its_declared_size_is_refused() {
    let data = vec![7u8; 1000];
    let mut out = Vec::new();
    assert!(copy_entry(&mut Cursor::new(&data), &mut out, 10, 10).is_err());
}

#[test]
fn an_entry_claiming_a_huge_ratio_is_refused_before_it_is_written() {
    let mut out = Vec::new();
    let error = copy_entry(&mut Cursor::new(Vec::<u8>::new()), &mut out, 10 << 30, 1000).unwrap_err();
    assert!(error.to_string().contains("refused"));
    assert!(out.is_empty());
}

#[test]
fn an_archive_whose_entries_total_too_much_is_refused() {
    assert!(archive_fits(1 << 20, 1 << 20));
    assert!(!archive_fits(100 << 30, 1 << 20));
}

#[test]
fn a_quarantined_file_is_kept_beside_its_path_and_the_path_is_freed() {
    let dir = scratch("quarantine");
    let path = dir.join("preferences.json");
    write_atomic(&path, b"{ broken").unwrap();
    let moved = quarantine(&path).unwrap();
    assert!(!path.exists());
    assert_eq!(fs::read(&moved).unwrap(), b"{ broken");
    assert!(moved.file_name().unwrap().to_string_lossy().starts_with("preferences.json.corrupt-"));
}

/// A source that never ends: every read fills the buffer with zeros.
struct Zeros;

impl Read for Zeros {
    fn read(&mut self, buf: &mut [u8]) -> io::Result<usize> {
        buf.fill(0);
        Ok(buf.len())
    }
}

#[test]
fn output_within_the_allowance_of_its_input_is_read() {
    let consumed = Arc::new(AtomicU64::new(1000));
    let mut bounded = Bounded::new(Zeros, consumed);
    let mut buf = vec![0u8; 1 << 20];
    assert_eq!(bounded.read(&mut buf).unwrap(), buf.len());
}

#[test]
fn output_far_beyond_its_input_is_refused() {
    // 1000 compressed bytes allow 100 000 bytes, plus the slack, and no more.
    let consumed = Arc::new(AtomicU64::new(1000));
    let mut bounded = Bounded::new(Zeros, Arc::clone(&consumed));
    let mut buf = vec![0u8; 1 << 20];
    let mut produced = 0u64;
    let error = loop {
        match bounded.read(&mut buf) {
            Ok(read) => {
                produced += read as u64;
                assert!(produced < 1 << 30, "the stream was not refused");
            }
            Err(error) => break error,
        }
    };
    assert!(error.to_string().contains("refused"), "{error}");
    assert!(produced <= 1000 * 100 + (16 << 20) + buf.len() as u64, "{produced}");
}

#[test]
fn a_counted_reader_counts_what_passes_through_it() {
    let (mut counted, count) = Counted::new(Cursor::new(vec![1u8; 300]));
    let mut out = Vec::new();
    io::copy(&mut counted, &mut out).unwrap();
    assert_eq!(count.load(std::sync::atomic::Ordering::Relaxed), 300);
}
