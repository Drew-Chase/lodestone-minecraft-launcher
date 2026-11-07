use std::path::Path;

use super::*;

#[test]
fn every_folder_hangs_from_the_root() {
    let paths = LauncherPaths::new("/data/lodestone");
    assert_eq!(paths.instances(), Path::new("/data/lodestone/instances"));
    assert_eq!(
        paths.instance("Dragon Survival v2"),
        Path::new("/data/lodestone/instances/Dragon Survival v2")
    );
    assert_eq!(paths.java("delta"), Path::new("/data/lodestone/meta/java/delta"));
}

#[test]
fn shared_downloads_sit_under_meta_once() {
    let paths = LauncherPaths::new("/data/lodestone");
    assert_eq!(paths.libraries(), Path::new("/data/lodestone/meta/libraries"));
    assert_eq!(paths.assets(), Path::new("/data/lodestone/meta/assets"));
    assert_eq!(paths.asset_indexes(), Path::new("/data/lodestone/meta/assets/indexes"));
    assert_eq!(paths.asset_objects(), Path::new("/data/lodestone/meta/assets/objects"));
    assert_eq!(
        paths.version_json("1.21.4"),
        Path::new("/data/lodestone/meta/versions/1.21.4/1.21.4.json")
    );
    assert_eq!(paths.client_jar("1.21.4"), Path::new("/data/lodestone/meta/versions/1.21.4/1.21.4.jar"));
    assert_eq!(paths.natives("1.21.4"), Path::new("/data/lodestone/meta/natives/1.21.4"));
    assert_eq!(paths.log_configs(), Path::new("/data/lodestone/meta/log_configs"));
}

#[test]
fn a_portable_launcher_keeps_its_folders_beside_its_file() {
    let root = portable_root(Path::new("/media/usb/Lodestone/lodestone_launcher.exe"), "pkg-1");
    assert_eq!(root, Path::new("/media/usb/Lodestone/lodestone-portable/pkg-1"));
}

#[test]
fn only_a_file_inside_a_macos_bundle_is_taken_as_one() {
    let inside = Path::new("/Volumes/USB/Lodestone.app/Contents/MacOS/lodestone_launcher");
    assert_eq!(app_bundle(inside).is_some(), cfg!(target_os = "macos"));
    assert!(app_bundle(Path::new("/media/usb/lodestone_launcher")).is_none());
}

#[test]
#[cfg(target_os = "macos")]
fn a_macos_portable_launcher_keeps_its_folders_beside_its_bundle() {
    let launcher = Path::new("/Volumes/USB/Lodestone.app/Contents/MacOS/lodestone_launcher");
    assert_eq!(portable_root(launcher, "pkg-1"), Path::new("/Volumes/USB/lodestone-portable/pkg-1"));
}
