use super::*;
use crate::support::{put, scratch};
use std::io::Read;

/// An instance with one jar in `mods` and one config file.
fn instance(name: &str) -> PathBuf {
    let root = scratch(name);
    put(&root, "mods/keep.jar", b"keep jar");
    put(&root, "config/a.toml", b"a = 1");
    root
}

/// The files a pack of `root` takes: the default folders, and its `mods/keep.jar`.
fn planned(root: &Path) -> Vec<Planned> {
    crate::server_pack::plan::plan(root, &crate::plan::Selection::new_export(), &[root.join("mods/keep.jar")]).expect("the plan builds")
}

/// The partial file or folder a write to `target` uses.
fn partial_of(target: &Path) -> PathBuf {
    PathBuf::from(format!("{}.partial", target.display()))
}

#[test]
fn a_folder_pack_holds_each_file_at_its_path() {
    let root = instance("server-pack-write-folder");
    let out = scratch("server-pack-write-folder-out");
    let target = out.join("Pack");
    let cancel = AtomicBool::new(false);
    let written = write(&planned(&root), &Output::Folder(target.clone()), &root, &cancel, &mut |_| {}).expect("the pack writes");
    assert_eq!(written, target);
    assert_eq!(fs::read(target.join("mods/keep.jar")).expect("the jar is written"), b"keep jar");
    assert_eq!(fs::read(target.join("config/a.toml")).expect("the config is written"), b"a = 1");
    assert!(!partial_of(&target).exists(), "the partial folder is renamed away");
}

#[test]
fn a_zip_pack_holds_the_files_and_no_manifest() {
    let root = instance("server-pack-write-zip");
    let out = scratch("server-pack-write-zip-out");
    let target = out.join("Pack.zip");
    let cancel = AtomicBool::new(false);
    write(&planned(&root), &Output::Zip(target.clone()), &root, &cancel, &mut |_| {}).expect("the pack writes");
    let mut archive = zip::ZipArchive::new(File::open(&target).expect("the zip exists")).expect("the zip opens");
    let mut names: Vec<String> = (0..archive.len())
        .map(|index| {
            archive
                .by_index(index)
                .expect("the entry reads")
                .name()
                .expect("the name reads")
                .to_string()
        })
        .collect();
    names.sort();
    assert_eq!(names, ["config/a.toml", "mods/keep.jar"]);
    let mut jar = Vec::new();
    archive
        .by_name("mods/keep.jar")
        .expect("the jar is in the zip")
        .read_to_end(&mut jar)
        .expect("it reads");
    assert_eq!(jar, b"keep jar");
    assert!(!partial_of(&target).exists(), "the partial file is renamed away");
}

#[test]
fn a_target_that_already_exists_is_refused_not_replaced() {
    let root = instance("server-pack-write-exists");
    let out = scratch("server-pack-write-exists-out");
    let target = out.join("Pack");
    fs::create_dir(&target).expect("the existing folder is made");
    let cancel = AtomicBool::new(false);
    let error = write(&planned(&root), &Output::Folder(target.clone()), &root, &cancel, &mut |_| {}).expect_err("an existing target is refused");
    assert!(
        matches!(error, Error::Io(ref io) if io.kind() == io::ErrorKind::AlreadyExists),
        "{error:?}"
    );
    assert!(target.is_dir(), "the existing folder is left alone");
}

#[test]
fn a_target_inside_the_instance_is_refused() {
    let root = instance("server-pack-write-inside");
    let target = root.join("server-pack");
    let cancel = AtomicBool::new(false);
    let error =
        write(&planned(&root), &Output::Folder(target.clone()), &root, &cancel, &mut |_| {}).expect_err("a target inside the instance is refused");
    assert!(
        matches!(error, Error::Io(ref io) if io.kind() == io::ErrorKind::InvalidInput),
        "{error:?}"
    );
    assert!(!target.exists());
}

#[test]
fn a_cancelled_write_leaves_no_output_behind() {
    let root = instance("server-pack-write-cancel");
    let out = scratch("server-pack-write-cancel-out");
    let cancel = AtomicBool::new(true);
    for output in [Output::Folder(out.join("Folder")), Output::Zip(out.join("Archive.zip"))] {
        let target = output.target().to_path_buf();
        let error = write(&planned(&root), &output, &root, &cancel, &mut |_| {}).expect_err("a cancelled write fails");
        assert!(matches!(error, Error::Cancelled), "{error:?}");
        assert!(!target.exists(), "no target is left");
        assert!(!partial_of(&target).exists(), "no partial output is left");
    }
}

#[test]
fn progress_reports_every_file_written() {
    let root = instance("server-pack-write-progress");
    let out = scratch("server-pack-write-progress-out");
    let files = planned(&root);
    let mut reports = Vec::new();
    let cancel = AtomicBool::new(false);
    write(&files, &Output::Zip(out.join("Progress.zip")), &root, &cancel, &mut |progress| {
        reports.push(progress)
    })
    .expect("the pack writes");
    let last = reports.last().expect("the write reports");
    assert_eq!(last.files, files.len());
    assert_eq!(last.total_files, files.len());
    assert_eq!(last.bytes, last.total_bytes);
}
