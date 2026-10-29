//! Builds fixture jars for the integration tests.

use std::fs::File;
use std::io::Write;
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicUsize, Ordering};
use std::time::{SystemTime, UNIX_EPOCH};
use zip::ZipWriter;
use zip::write::SimpleFileOptions;

static NEXT_ID: AtomicUsize = AtomicUsize::new(0);

/// Creates a jar with the given `(archive path, content)` entries in a fresh
/// temp directory, and returns its path. The directory is left for the OS temp
/// dir to clean up.
pub(crate) fn jar(case: &str, entries: &[(&str, &str)]) -> PathBuf {
    let stamp = SystemTime::now().duration_since(UNIX_EPOCH).expect("clock is sane").as_nanos();
    let dir = std::env::temp_dir().join(format!(
        "lodestone-loaders-test-{}-{}-{}-{}",
        case,
        std::process::id(),
        NEXT_ID.fetch_add(1, Ordering::Relaxed),
        stamp,
    ));
    std::fs::create_dir_all(&dir).expect("temp dir creates");
    let path = dir.join("mod.jar");
    write_jar(&path, entries);
    path
}

fn write_jar(path: &Path, entries: &[(&str, &str)]) {
    let file = File::create(path).expect("jar file creates");
    let mut archive = ZipWriter::new(file);
    for (name, content) in entries {
        archive.start_file(*name, SimpleFileOptions::default()).expect("entry starts");
        archive.write_all(content.as_bytes()).expect("entry writes");
    }
    archive.finish().expect("jar finishes");
}
