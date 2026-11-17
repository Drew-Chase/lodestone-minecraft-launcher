use std::fs;

use super::*;

fn scratch(name: &str) -> PathBuf {
    let dir = std::env::temp_dir().join(format!("lodestone-editing-{name}-{}", std::process::id()));
    let _ = fs::remove_dir_all(&dir);
    fs::create_dir_all(&dir).expect("the scratch folder");
    dir
}

#[test]
fn a_text_file_is_read_with_its_encoding_and_its_stamp() {
    let root = scratch("read");
    let file = root.join("a.toml");
    fs::write(&file, "a = 1\n").expect("a file");
    let loaded = read_text(&file, "a.toml".to_string()).expect("the file is text");
    assert_eq!(
        (loaded.path.as_str(), loaded.text.as_str(), loaded.encoding),
        ("a.toml", "a = 1\n", Encoding::Utf8)
    );
    assert_eq!(Some(loaded.stamp), stamp_of(&file));
    assert_eq!(loaded.stamp.length, 6);
    let _ = fs::remove_dir_all(&root);
}

#[test]
fn a_file_that_is_not_text_too_large_or_gone_is_refused_with_its_reason() {
    let root = scratch("refuse");
    let binary = root.join("level.dat");
    fs::write(&binary, [0x0A, 0x00, 0x00, 0x08, 0x00]).expect("a file");
    assert_eq!(read_text(&binary, "level.dat".to_string()), Err(ReadError::NotText));

    let large = root.join("debug.log");
    fs::write(&large, vec![b'a'; (MAX_BYTES + 1) as usize]).expect("a file");
    assert_eq!(read_text(&large, "debug.log".to_string()), Err(ReadError::TooLarge));

    assert!(matches!(
        read_text(&root.join("missing.txt"), "missing.txt".to_string()),
        Err(ReadError::Io(_))
    ));
    assert_eq!(stamp_of(&root.join("missing.txt")), None);
    let _ = fs::remove_dir_all(&root);
}

#[test]
fn a_save_writes_the_file_and_returns_its_new_stamp() {
    let root = scratch("save");
    let file = root.join("a.toml");
    fs::write(&file, "a = 1").expect("a file");
    let known = stamp_of(&file).expect("the file's stamp");
    let stamp = write_text(&file, b"a = 12", known, false).expect("the save");
    assert_eq!(fs::read(&file).expect("the file"), b"a = 12");
    assert_eq!(stamp.length, 6);
    assert_eq!(Some(stamp), stamp_of(&file));
    let _ = fs::remove_dir_all(&root);
}

#[test]
fn a_file_that_changed_on_disk_is_not_overwritten_unless_forced() {
    let root = scratch("conflict");
    let file = root.join("a.toml");
    fs::write(&file, "a = 1").expect("a file");
    let known = stamp_of(&file).expect("the file's stamp");
    // Something else rewrites the file, to another length, after the editor read it.
    fs::write(&file, "a = 1000").expect("another write");
    assert_eq!(write_text(&file, b"mine", known, false), Err(SaveError::Changed));
    assert_eq!(fs::read(&file).expect("the file"), b"a = 1000", "what is on disk is left alone");
    assert!(write_text(&file, b"mine", known, true).is_ok());
    assert_eq!(fs::read(&file).expect("the file"), b"mine");
    let _ = fs::remove_dir_all(&root);
}

#[test]
fn a_file_that_is_gone_is_written_again() {
    let root = scratch("gone");
    let file = root.join("a.toml");
    fs::write(&file, "a = 1").expect("a file");
    let known = stamp_of(&file).expect("the file's stamp");
    fs::remove_file(&file).expect("the file goes");
    assert!(write_text(&file, b"a = 1", known, false).is_ok());
    assert!(file.is_file());
    let _ = fs::remove_dir_all(&root);
}
