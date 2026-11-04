//! Tests for `app/reveal.rs`, kept out of the source file. They are compiled as the `tests` child module of `reveal`.

#![cfg(all(unix, not(target_os = "macos")))]

use std::path::Path;

use super::file_uri;

#[test]
fn a_path_is_escaped_into_a_file_address() {
    assert_eq!(file_uri(Path::new("/home/me/My Mods/a+b.jar")), "file:///home/me/My%20Mods/a%2Bb.jar");
    assert_eq!(file_uri(Path::new("/tmp/mod-1.0_x~.jar")), "file:///tmp/mod-1.0_x~.jar");
    assert_eq!(
        file_uri(Path::new("/tmp/'quoted'.jar")),
        "file:///tmp/%27quoted%27.jar",
        "quotes cannot end the bus argument early"
    );
}
