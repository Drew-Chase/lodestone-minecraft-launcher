use std::sync::atomic::AtomicBool;

use super::*;

/// A scratch folder unique to this test run and tag.
fn scratch(tag: &str) -> PathBuf {
    let dir = std::env::temp_dir().join(format!("lodestone-copy-{tag}-{}", std::process::id()));
    std::fs::remove_dir_all(&dir).ok();
    std::fs::create_dir_all(&dir).expect("scratch folder created");
    dir
}

/// Writes `body` to `relative` under `root`, making the folders it needs.
fn put(root: &Path, relative: &str, body: &str) {
    let path = root.join(relative);
    std::fs::create_dir_all(path.parent().expect("a file has a parent")).expect("folders made");
    std::fs::write(path, body).expect("file written");
}

/// The paths a plan takes, in order.
fn paths(planned: &[Planned]) -> Vec<&str> {
    planned.iter().map(|file| file.path.as_str()).collect()
}

#[test]
fn a_plan_keeps_the_game_files_and_leaves_out_the_launchers_own() {
    let root = scratch("plan");
    put(&root, "mods/sodium.jar", "jar");
    put(&root, "config/options.toml", "x=1");
    put(&root, "saves/World/level.dat", "world");
    put(&root, "screenshots/shot.png", "png");
    put(&root, "logs/latest.log", "log");
    put(&root, "crash-reports/crash.txt", "crash");
    put(&root, "versions/1.21.1/1.21.1.jar", "client");
    put(&root, "libraries/lib.jar", "lib");
    put(&root, "launcher_profiles.json", "{}");
    put(&root, "launcher_accounts_microsoft_store.json", "{}");
    put(&root, "usercache.json", "{}");
    put(&root, "minecraftinstance.json", "{}");
    put(&root, "profileImage/cover.png", "png");
    put(&root, "journeymap/data.bin", "bin");
    put(&root, "mods/.installing", "marker");

    let skip = vec!["minecraftinstance.json".to_string(), "profileImage".to_string()];
    let planned = plan(&root, &skip).expect("the folder can be read");
    assert_eq!(
        paths(&planned),
        [
            "config/options.toml",
            "journeymap/data.bin",
            "mods/sodium.jar",
            "saves/World/level.dat",
            "screenshots/shot.png",
        ]
    );
    assert_eq!(total_bytes(&planned), 3 + 3 + 3 + 5 + 3);
    std::fs::remove_dir_all(&root).ok();
}

#[cfg(unix)]
#[test]
fn a_link_to_a_file_is_copied_and_a_link_to_a_folder_is_left_out() {
    let root = scratch("links");
    put(&root, "shared/servers.dat", "servers");
    put(&root, "elsewhere/inner.txt", "inner");
    std::fs::create_dir_all(root.join("game")).expect("game folder made");
    std::os::unix::fs::symlink(root.join("shared/servers.dat"), root.join("game/servers.dat")).expect("file link");
    std::os::unix::fs::symlink(root.join("elsewhere"), root.join("game/linked-folder")).expect("folder link");

    let planned = plan(&root.join("game"), &[]).expect("the folder can be read");
    assert_eq!(paths(&planned), ["servers.dat"]);
    std::fs::remove_dir_all(&root).ok();
}

#[test]
fn a_copy_recreates_the_paths_and_counts_what_it_copied() {
    let root = scratch("copy-source");
    put(&root, "mods/sodium.jar", "jar");
    put(&root, "config/a/b.toml", "x=1");
    let planned = plan(&root, &[]).expect("the folder can be read");
    let destination = scratch("copy-destination");

    let report = copy_into(&planned, &destination, &AtomicBool::new(false)).expect("the copy runs");
    assert_eq!(report, Report { copied: 2, failed: 0 });
    assert_eq!(std::fs::read_to_string(destination.join("config/a/b.toml")).expect("copied"), "x=1");
    assert!(destination.join("mods/sodium.jar").is_file());
    std::fs::remove_dir_all(&root).ok();
    std::fs::remove_dir_all(&destination).ok();
}

#[test]
fn a_file_that_cannot_be_copied_is_counted_and_the_rest_are_copied() {
    let root = scratch("unreadable");
    put(&root, "mods/sodium.jar", "jar");
    let mut planned = plan(&root, &[]).expect("the folder can be read");
    planned.push(Planned {
        path: "config/gone.toml".to_string(),
        source: root.join("missing.toml"),
        size: 0,
    });
    let destination = scratch("unreadable-destination");

    let report = copy_into(&planned, &destination, &AtomicBool::new(false)).expect("the copy runs");
    assert_eq!(report, Report { copied: 1, failed: 1 });
    std::fs::remove_dir_all(&root).ok();
    std::fs::remove_dir_all(&destination).ok();
}

#[test]
fn an_abandoned_copy_stops_before_its_next_file() {
    let root = scratch("abandoned");
    put(&root, "mods/sodium.jar", "jar");
    let planned = plan(&root, &[]).expect("the folder can be read");
    let destination = scratch("abandoned-destination");

    let error = copy_into(&planned, &destination, &AtomicBool::new(true)).expect_err("the copy is refused");
    assert_eq!(error.kind(), std::io::ErrorKind::Interrupted);
    assert!(!destination.join("mods/sodium.jar").exists());
    std::fs::remove_dir_all(&root).ok();
    std::fs::remove_dir_all(&destination).ok();
}
