use std::ffi::OsStr;
use std::path::{Path, PathBuf};

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

#[test]
fn an_unset_or_blank_app_directory_changes_nothing() {
    assert_eq!(app_directory(None), None);
    assert_eq!(app_directory(Some(OsStr::new(""))), None);
    assert_eq!(app_directory(Some(OsStr::new("   "))), None);
}

#[test]
fn an_app_directory_is_kept_as_an_absolute_path() {
    let dir = std::env::temp_dir().join("lodestone-dev-env");
    assert_eq!(app_directory(Some(dir.as_os_str())), Some(dir.clone()));
}

#[test]
fn a_development_root_keeps_its_config_data_and_cache_under_it() {
    let roots = FixedRoots::under(PathBuf::from("/dev/lodestone-env"));
    assert_eq!(roots.root, Path::new("/dev/lodestone-env"));
    assert_eq!(roots.config, Path::new("/dev/lodestone-env/config"));
    assert_eq!(roots.data, Path::new("/dev/lodestone-env/data"));
    assert_eq!(roots.cache, Path::new("/dev/lodestone-env/cache"));
}

#[test]
fn the_instances_sit_inside_the_root_until_a_folder_is_chosen() {
    let paths = LauncherPaths::new("/data/lodestone");
    assert_eq!(paths.instances_chosen(), None);
    let chosen = paths.clone().with_instances(Some(PathBuf::from("/games/instances")));
    assert_eq!(chosen.instances(), Path::new("/games/instances"));
    assert_eq!(chosen.instance("Pack"), Path::new("/games/instances/Pack"));
    assert_eq!(chosen.instances_chosen(), Some(Path::new("/games/instances")));
}

#[test]
fn a_root_change_keeps_a_chosen_instances_folder_and_moves_the_default_one() {
    let inside = LauncherPaths::new("/data/lodestone");
    let moved = inside.with_root("/mnt/data");
    assert_eq!(
        moved.instances(),
        Path::new("/mnt/data/instances"),
        "the default folder moves with the root"
    );

    let outside = LauncherPaths::new("/data/lodestone").with_instances(Some(PathBuf::from("/games/instances")));
    let moved = outside.with_root("/mnt/data");
    assert_eq!(moved.instances(), Path::new("/games/instances"), "a chosen folder stays where it is");
    assert_eq!(moved.root(), Path::new("/mnt/data"));
}

#[test]
fn the_logs_and_cache_show_their_chosen_folder_or_the_default_one() {
    let paths = LauncherPaths::new("/data/lodestone");
    assert_eq!(paths.logs_chosen(), None);
    assert_eq!(paths.cache_chosen(), None);

    let chosen = paths
        .with_logs(Some(PathBuf::from("/var/log/lodestone")))
        .with_cache(Some(PathBuf::from("/var/cache/lodestone")));
    assert_eq!(chosen.logs(), Path::new("/var/log/lodestone"));
    assert_eq!(chosen.cache(), Path::new("/var/cache/lodestone"));
    assert_eq!(chosen.logs_chosen(), Some(Path::new("/var/log/lodestone")));
    assert_eq!(chosen.cache_chosen(), Some(Path::new("/var/cache/lodestone")));
}

#[test]
fn a_root_change_keeps_the_logs_and_cache_the_user_chose() {
    let paths = LauncherPaths::new("/data/lodestone").with_logs(Some(PathBuf::from("/var/log/lodestone")));
    let moved = paths.with_root("/mnt/data");
    assert_eq!(moved.logs_chosen(), Some(Path::new("/var/log/lodestone")));
}
