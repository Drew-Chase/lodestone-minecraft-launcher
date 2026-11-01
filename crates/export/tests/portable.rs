use std::fs::File;
use std::io::Read;
use std::sync::atomic::AtomicBool;

use super::*;
use crate::format::Package;
use crate::format::lodestone::read_manifest;
use crate::progress::{Control, Progress};
use crate::support::{entry, metadata, put, scratch};

/// A package holding one config file, for the portable tests to carry.
fn package(root: &Path) -> (crate::format::Metadata, Vec<crate::link::Entry>) {
    let entries = vec![entry(
        "config/options.toml",
        put(root, "config/options.toml", b"fov=70"),
        false,
        Default::default(),
    )];
    (metadata(), entries)
}

#[test]
fn a_written_launcher_starts_with_the_copied_bytes_and_carries_the_package_after_them() {
    let root = scratch("portable-binary");
    let executable = put(&root, "launcher", b"launcher-code-bytes");
    let (metadata, entries) = package(&root);
    let package = Package {
        id: "portable-id".to_string(),
        exported_at: 42,
        metadata: &metadata,
        entries: &entries,
        bundle_linked: false,
    };
    let cancel = AtomicBool::new(false);
    let mut report = |_: Progress| {};
    let mut control = Control::new(&cancel, 1, 7, &mut report);
    let target = root.join("Enhanced Worlds");
    write_binary(&executable, &target, &package, &mut control).expect("writes");

    let written = std::fs::read(&target).expect("reads");
    assert!(written.starts_with(b"launcher-code-bytes"));
    assert!(written.ends_with(MAGIC));

    let located = locate(&target).expect("locates").expect("the package is found");
    let Located::Appended { offset, len } = located.clone() else {
        panic!("the package is appended to the launcher");
    };
    assert_eq!(offset, b"launcher-code-bytes".len() as u64);
    let mut carried = Vec::new();
    open(&target, &located).expect("opens").read_to_end(&mut carried).expect("reads");
    assert_eq!(carried.len() as u64, len);
    let manifest = read_manifest(&carried[..]).expect("the carried package reads");
    assert_eq!(manifest.id, "portable-id");
}

#[test]
fn a_launcher_without_a_footer_carries_no_package() {
    let root = scratch("portable-plain");
    let executable = put(&root, "launcher", b"just a launcher");
    assert_eq!(locate(&executable).expect("locates"), None);
}

#[test]
fn a_footer_that_claims_more_than_the_file_holds_is_not_a_package() {
    let root = scratch("portable-short");
    let mut bytes = 9_999_999u64.to_le_bytes().to_vec();
    bytes.extend_from_slice(MAGIC);
    let executable = put(&root, "launcher", &bytes);
    assert_eq!(locate(&executable).expect("locates"), None);
}

#[test]
fn the_sidecar_sits_in_the_app_bundle_beside_the_launcher() {
    let executable = Path::new("/Applications/Enhanced Worlds.app/Contents/MacOS/lodestone");
    assert_eq!(
        sidecar(executable),
        Some(PathBuf::from("/Applications/Enhanced Worlds.app/Contents/Resources/payload.lodestone"))
    );
}

#[test]
fn an_app_bundle_holds_the_launcher_its_plist_and_the_package() {
    let root = scratch("portable-app");
    let executable = put(&root, "launcher", b"macos-launcher");
    let (metadata, entries) = package(&root);
    let package = Package {
        id: "app-id".to_string(),
        exported_at: 7,
        metadata: &metadata,
        entries: &entries,
        bundle_linked: false,
    };
    let cancel = AtomicBool::new(false);
    let mut report = |_: Progress| {};
    let mut control = Control::new(&cancel, 1, 7, &mut report);
    let target = root.join("Enhanced Worlds.zip");
    write_app(
        &executable,
        &target,
        "Enhanced Worlds",
        "dev.lodestone.portable.app-id",
        &package,
        &mut control,
    )
    .expect("writes");

    let mut archive = zip::ZipArchive::new(File::open(&target).expect("opens")).expect("reads");
    let mut plist = String::new();
    archive
        .by_name("Enhanced Worlds.app/Contents/Info.plist")
        .expect("plist")
        .read_to_string(&mut plist)
        .expect("reads");
    assert!(plist.contains("<string>dev.lodestone.portable.app-id</string>"));
    assert!(plist.contains("<string>lodestone</string>"));
    let program = archive.by_name("Enhanced Worlds.app/Contents/MacOS/lodestone").expect("program");
    assert_eq!(program.unix_mode().map(|mode| mode & 0o777), Some(0o755));
    drop(program);
    let mut payload = Vec::new();
    archive
        .by_name("Enhanced Worlds.app/Contents/Resources/payload.lodestone")
        .expect("payload")
        .read_to_end(&mut payload)
        .expect("reads");
    assert_eq!(read_manifest(&payload[..]).expect("reads").id, "app-id");
}
