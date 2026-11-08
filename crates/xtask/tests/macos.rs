use super::*;
use std::fs;

/// A fresh folder under the system temp directory, named for the test.
fn scratch(name: &str) -> PathBuf {
    let dir = std::env::temp_dir().join(format!("xtask-{name}-{}", std::process::id()));
    let _ = fs::remove_dir_all(&dir);
    fs::create_dir_all(&dir).expect("creating the scratch folder");
    dir
}

fn words(list: &[&str]) -> Vec<String> {
    list.iter().map(|word| word.to_string()).collect()
}

#[test]
fn version_is_three_numbers() {
    assert!(is_version("0.1.0"));
    assert!(is_version("12.0.345"));
    assert!(!is_version("0.1"));
    assert!(!is_version("v0.1.0"));
    assert!(!is_version("0.1.0-rc.1"));
    assert!(!is_version("0..1"));
    assert!(!is_version(""));
}

#[test]
fn options_need_binary_version_and_out() {
    let parsed = options(&words(&["--binary", "bin", "--version", "0.1.0", "--out", "dist"])).expect("valid options");
    assert_eq!(parsed.binary, PathBuf::from("bin"));
    assert_eq!(parsed.icon, PathBuf::from("res/icon/icon.icns"));
    assert_eq!(parsed.dmg, None);
    assert!(options(&words(&["--binary", "bin", "--version", "0.1.0"])).is_err());
    assert!(options(&words(&["--binary", "bin", "--version", "v0.1.0", "--out", "dist"])).is_err());
}

#[test]
fn options_read_the_icon_and_disk_image() {
    let parsed = options(&words(&[
        "--binary",
        "bin",
        "--version",
        "0.2.0",
        "--out",
        "dist",
        "--icon",
        "mark.icns",
        "--dmg",
        "dist/app.dmg",
    ]))
    .expect("valid options");
    assert_eq!(parsed.icon, PathBuf::from("mark.icns"));
    assert_eq!(parsed.dmg, Some(PathBuf::from("dist/app.dmg")));
}

#[test]
fn plist_names_the_executable_and_version() {
    let plist = info_plist("0.1.0");
    assert!(plist.contains("<key>CFBundleExecutable</key>\n    <string>lodestone_launcher</string>"));
    assert!(plist.contains("<key>CFBundleShortVersionString</key>\n    <string>0.1.0</string>"));
    assert!(plist.contains("<key>CFBundleVersion</key>\n    <string>0.1.0</string>"));
    assert!(plist.contains(BUNDLE_ID));
}

#[test]
fn bundle_has_the_expected_layout() {
    let root = scratch("layout");
    let binary = root.join("lodestone_launcher");
    let icon = root.join("mark.icns");
    fs::write(&binary, b"binary").expect("writing the binary");
    fs::write(&icon, b"icon").expect("writing the icon");
    let options = Options {
        binary,
        icon,
        version: "0.1.0".into(),
        out: root.join("out"),
        dmg: None,
    };

    let app = write_bundle(&options).expect("writing the bundle");

    assert!(app.ends_with("Lodestone.app"));
    assert_eq!(
        fs::read(app.join("Contents/MacOS/lodestone_launcher")).expect("the binary"),
        b"binary".to_vec()
    );
    assert_eq!(fs::read(app.join("Contents/Resources/AppIcon.icns")).expect("the icon"), b"icon".to_vec());
    let plist = fs::read_to_string(app.join("Contents/Info.plist")).expect("the plist");
    assert!(plist.contains("<string>0.1.0</string>"));
    let _ = fs::remove_dir_all(&root);
}

#[test]
fn bundle_replaces_a_stale_one() {
    let root = scratch("stale");
    let binary = root.join("lodestone_launcher");
    let icon = root.join("mark.icns");
    fs::write(&binary, b"new").expect("writing the binary");
    fs::write(&icon, b"icon").expect("writing the icon");
    let stale = root.join("out/Lodestone.app/Contents/stale.txt");
    fs::create_dir_all(stale.parent().expect("a parent folder")).expect("the stale folder");
    fs::write(&stale, b"old").expect("writing the stale file");
    let options = Options {
        binary,
        icon,
        version: "0.1.0".into(),
        out: root.join("out"),
        dmg: None,
    };

    write_bundle(&options).expect("writing the bundle");

    assert!(!stale.exists());
    let _ = fs::remove_dir_all(&root);
}

#[cfg(unix)]
#[test]
fn the_binary_inside_the_bundle_is_executable() {
    use std::os::unix::fs::PermissionsExt;
    let root = scratch("mode");
    let binary = root.join("lodestone_launcher");
    let icon = root.join("mark.icns");
    fs::write(&binary, b"binary").expect("writing the binary");
    fs::write(&icon, b"icon").expect("writing the icon");
    let options = Options {
        binary,
        icon,
        version: "0.1.0".into(),
        out: root.join("out"),
        dmg: None,
    };

    let app = write_bundle(&options).expect("writing the bundle");

    let mode = fs::metadata(app.join("Contents/MacOS/lodestone_launcher"))
        .expect("the binary")
        .permissions()
        .mode();
    assert_eq!(mode & 0o777, 0o755);
    let _ = fs::remove_dir_all(&root);
}

#[test]
fn the_image_is_sized_with_room_to_spare() {
    assert_eq!(image_size_mb(0), 64);
    assert_eq!(image_size_mb(1), 65);
    assert_eq!(image_size_mb(100 * 1024 * 1024), 100 + 50 + 64);
}

#[test]
fn staged_files_are_summed_through_their_folders() {
    let root = scratch("staged-bytes");
    fs::create_dir_all(root.join("app/Contents/MacOS")).expect("folders");
    fs::write(root.join("app/Contents/MacOS/lodestone_launcher"), vec![0u8; 1000]).expect("file");
    fs::write(root.join("app/Info.plist"), vec![0u8; 2345]).expect("file");
    assert_eq!(dir_bytes(&root).expect("sums"), 3345);
    fs::remove_dir_all(&root).ok();
}

#[cfg(unix)]
#[test]
fn symbolic_links_add_no_bytes() {
    let root = scratch("staged-links");
    fs::write(root.join("file"), vec![0u8; 10]).expect("file");
    std::os::unix::fs::symlink("/Applications", root.join("Applications")).expect("link");
    assert_eq!(dir_bytes(&root).expect("sums"), 10);
    fs::remove_dir_all(&root).ok();
}
