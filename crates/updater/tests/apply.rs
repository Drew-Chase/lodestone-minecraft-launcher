use super::*;
use std::fs;

/// A fresh folder under the system temp directory, named for the test.
fn scratch(name: &str) -> PathBuf {
    let dir = std::env::temp_dir().join(format!("lodestone-updater-apply-{name}-{}", std::process::id()));
    let _ = fs::remove_dir_all(&dir);
    fs::create_dir_all(&dir).expect("creating the scratch folder");
    dir
}

/// Bytes that start like a Linux program, followed by `body`.
fn program(body: &[u8]) -> Vec<u8> {
    let mut bytes = b"\x7fELF".to_vec();
    bytes.extend_from_slice(body);
    bytes
}

/// Packs `file` from `source` into a gzipped tar at `archive`, with tar itself.
fn pack(source: &Path, file: &str, archive: &Path) {
    let status = Command::new("tar")
        .arg("-czf")
        .arg(archive)
        .arg("-C")
        .arg(source)
        .arg(file)
        .status()
        .expect("running tar");
    assert!(status.success(), "tar could not pack the test archive");
}

#[cfg(unix)]
#[test]
fn the_program_starts_only_after_the_process_it_waits_for_has_ended() {
    let dir = scratch("relaunch");
    let marker = dir.join("started");
    let mut waited = Command::new("sleep").arg("1").spawn().expect("a process to wait for");

    relaunch_after_exit(waited.id(), Path::new("/usr/bin/touch"), &[marker.as_path()]).expect("the relaunch command");
    // The waited process is still running, so the program has not started yet.
    assert!(!marker.exists());

    waited.wait().expect("waiting for the process");
    // The shell starts the program a moment after the process ends, so the test allows it a few seconds.
    let deadline = std::time::Instant::now() + std::time::Duration::from_secs(10);
    while !marker.exists() && std::time::Instant::now() < deadline {
        std::thread::sleep(std::time::Duration::from_millis(50));
    }
    assert!(marker.exists(), "the program did not start after the process ended");
    let _ = fs::remove_dir_all(&dir);
}

#[test]
fn a_folder_that_takes_new_files_is_writable() {
    let dir = scratch("writable");
    assert!(can_write_to(&dir));
    assert!(!can_write_to(&dir.join("missing")));
    let _ = fs::remove_dir_all(&dir);
}

#[test]
fn a_new_executable_replaces_the_old_one_and_keeps_the_staged_copy() {
    let dir = scratch("replace");
    let staged = dir.join("staged");
    let target = dir.join("lodestone_launcher");
    fs::write(&staged, program(b"new")).expect("writing the staged file");
    fs::write(&target, b"old").expect("writing the running file");

    replace_executable(&staged, &target).expect("placing the update");

    assert_eq!(fs::read(&target).expect("reading the target"), program(b"new"));
    assert!(staged.is_file());
    assert!(!dir.join(".lodestone_launcher.update").exists());
    let _ = fs::remove_dir_all(&dir);
}

#[test]
fn an_archive_yields_its_executable_and_refuses_what_is_not_a_program() {
    let dir = scratch("unpack");
    let source = dir.join("source");
    fs::create_dir_all(&source).expect("a source folder");
    fs::write(source.join("lodestone_launcher"), program(b"binary")).expect("the executable");
    let archive = dir.join("Lodestone-v0.2.0-linux-x86_64.tar.gz");
    pack(&source, "lodestone_launcher", &archive);

    let unpacked = extract_executable(&archive, &dir.join("unpacked")).expect("unpacking the executable");
    assert_eq!(fs::read(&unpacked).expect("reading the executable"), program(b"binary"));

    fs::write(source.join("lodestone_launcher"), b"not a program").expect("a fake executable");
    let fake = dir.join("fake.tar.gz");
    pack(&source, "lodestone_launcher", &fake);
    assert!(matches!(extract_executable(&fake, &dir.join("fake")), Err(Error::Unpack(_))));
    let _ = fs::remove_dir_all(&dir);
}

#[cfg(target_os = "macos")]
#[test]
fn a_new_app_bundle_replaces_the_old_one() {
    let dir = scratch("bundle");
    let staged = dir.join("staged").join("Lodestone.app");
    fs::create_dir_all(staged.join("Contents/MacOS")).expect("the staged bundle");
    fs::write(staged.join("Contents/MacOS/lodestone_launcher"), program(b"new")).expect("the new launcher");
    let bundle = dir.join("Lodestone.app");
    fs::create_dir_all(bundle.join("Contents/MacOS")).expect("the old bundle");
    fs::write(bundle.join("Contents/MacOS/lodestone_launcher"), b"old").expect("the old launcher");

    replace_bundle(&staged, &bundle).expect("placing the bundle");

    assert_eq!(
        fs::read(bundle.join("Contents/MacOS/lodestone_launcher")).expect("reading the launcher"),
        program(b"new")
    );
    assert!(!dir.join(".Lodestone.app.previous").exists());
    assert!(!dir.join(".Lodestone.app.update").exists());
    let _ = fs::remove_dir_all(&dir);
}
