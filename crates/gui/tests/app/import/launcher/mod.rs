use std::collections::BTreeSet;
use std::path::Path;

use super::*;
use crate::app::install::copy;

/// A scratch folder unique to this test run and tag.
fn scratch(tag: &str) -> PathBuf {
    let dir = std::env::temp_dir().join(format!("lodestone-launcher-{tag}-{}", std::process::id()));
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

fn meta(name: &str, minecraft: &str, loader: Loader, loader_version: Option<&str>) -> Meta {
    Meta {
        name: name.to_string(),
        minecraft: minecraft.to_string(),
        loader,
        loader_version: loader_version.map(str::to_string),
        game_dir: PathBuf::from("/game"),
        skip: Vec::new(),
    }
}

#[test]
fn the_launchers_have_distinct_keys_and_formats_that_map_back() {
    let keys: BTreeSet<&str> = Launcher::ALL.iter().map(|launcher| launcher.key()).collect();
    assert_eq!(keys.len(), Launcher::ALL.len());
    for launcher in Launcher::ALL {
        assert_eq!(Launcher::from_key(launcher.key()), Some(launcher));
        assert_eq!(Launcher::from_format(launcher.format()), Some(launcher));
    }
    assert_eq!(Launcher::from_format(ImportFormat::Mrpack), None);
}

#[test]
fn a_summary_names_the_version_and_the_loader_version() {
    assert_eq!(summary(&meta("Vanilla", "1.20.1", Loader::Vanilla, None)), "1.20.1 · VANILLA");
    assert_eq!(
        summary(&meta("Fabric", "1.21.1", Loader::Fabric, Some("0.16.10"))),
        "1.21.1 · FABRIC 0.16.10"
    );
}

#[test]
fn loader_names_map_to_the_loaders_this_app_installs() {
    assert_eq!(loader_named("NeoForge"), Some(Loader::NeoForge));
    assert_eq!(loader_named("fabric"), Some(Loader::Fabric));
    assert_eq!(loader_named("quilt"), Some(Loader::Quilt));
    assert_eq!(loader_named("forge"), Some(Loader::Forge));
    assert_eq!(loader_named(""), Some(Loader::Vanilla));
    assert_eq!(loader_named("liteloader"), None, "LiteLoader cannot be installed here");
}

#[test]
fn a_forge_build_takes_the_minecraft_version_in_front() {
    assert_eq!(forge_version("1.20.1", "47.2.0"), "1.20.1-47.2.0");
    assert_eq!(forge_version("1.20.1", "1.20.1-47.2.0"), "1.20.1-47.2.0");
}

#[test]
fn a_located_folder_that_holds_no_instances_is_refused_and_saves_nothing() {
    let folder = scratch("empty-locate");
    let refused = locate(Launcher::AtLauncher, folder.clone());
    assert_eq!(
        refused.err(),
        Some(lang::t_with("import.launcher.not_in_folder", &[("name", Launcher::AtLauncher.name())]).to_string())
    );
    std::fs::remove_dir_all(&folder).ok();
}

#[test]
fn an_instance_that_cannot_be_read_gives_its_folder_name_and_the_reason() {
    let row = instance(
        Launcher::Prism,
        "/instances/Old Pack".to_string(),
        "Old Pack".to_string(),
        Err("no version".to_string()),
    );
    assert_eq!(row.name, "Old Pack");
    assert_eq!(row.summary, Err("no version".to_string()));
}

#[test]
fn preparing_a_prism_instance_copies_its_game_folder_and_records_the_import() {
    let root = scratch("prepare");
    let folder = root.join("instances").join("Better Pack");
    put(
        &folder,
        "mmc-pack.json",
        r#"{"components":[{"uid":"net.minecraft","version":"1.21.1"},{"uid":"net.neoforged","version":"21.1.251"}]}"#,
    );
    put(&folder, "instance.cfg", "name=Better Pack\n");
    put(&folder, "minecraft/mods/sodium.jar", "jar");
    put(&folder, "minecraft/saves/World/level.dat", "world");
    let source = folder.to_string_lossy().into_owned();
    let paths = LauncherPaths::new(root.join("launcher"));

    let prepared = prepare(
        &[Pick {
            launcher: Launcher::Prism,
            source: source.clone(),
        }],
        &paths,
        &[],
    )
    .expect("the instance prepares");

    assert_eq!(prepared.len(), 1);
    let one = &prepared[0];
    assert_eq!(one.request.name, "Better Pack");
    assert_eq!(one.request.mc_version, "1.21.1");
    assert_eq!(one.request.loader, Loader::NeoForge);
    assert_eq!(one.request.loader_version.as_deref(), Some("21.1.251"));
    assert_eq!(one.record.format, ImportFormat::PrismLauncher);
    assert_eq!(one.record.origin, ImportOrigin::Launcher);
    assert_eq!(one.record.source, source);
    assert_eq!(one.record.bytes, 3 + 5, "the record counts what the copy takes");
    let payload = one.request.modpack.as_ref().expect("the install carries the folder");
    assert!(matches!(
        &payload.unpacks[0].origin,
        Origin::Folder { root: copied, skip } if copied == &folder.join("minecraft") && skip.is_empty()
    ));
    assert_eq!(copy::plan(&folder.join("minecraft"), &[]).expect("readable").len(), 2);
    std::fs::remove_dir_all(&root).ok();
}

#[test]
fn two_instances_with_one_name_get_different_folders() {
    let root = scratch("duplicates");
    let first = root.join("one");
    let second = root.join("two");
    for folder in [&first, &second] {
        put(folder, "mmc-pack.json", r#"{"components":[{"uid":"net.minecraft","version":"1.20.1"}]}"#);
        put(folder, "instance.cfg", "name=Same\n");
        put(folder, "minecraft/options.txt", "x");
    }
    let picks = [first, second]
        .iter()
        .map(|folder| Pick {
            launcher: Launcher::Prism,
            source: folder.to_string_lossy().into_owned(),
        })
        .collect::<Vec<_>>();
    let paths = LauncherPaths::new(root.join("launcher"));
    let prepared = prepare(&picks, &paths, &[]).expect("both prepare");
    assert_ne!(prepared[0].request.directory_name, prepared[1].request.directory_name);
    std::fs::remove_dir_all(&root).ok();
}

#[test]
fn the_default_folders_of_prism_sit_in_the_data_folder() {
    let folders = roots::default_folders(Launcher::Prism);
    if let Some(data) = dirs::data_dir() {
        assert!(folders.contains(&data.join("PrismLauncher")));
    }
    assert!(roots::default_folders(Launcher::MultiMc).is_empty(), "MultiMC is only found when located");
}

/// Looks at this machine's own launchers and prints what it finds. It reads the folders and writes nothing, so it is run by
/// hand: `cargo test -p lodestone_launcher scan_finds -- --ignored --nocapture`.
#[tokio::test(flavor = "multi_thread")]
#[ignore = "reads this machine's launcher folders; run by hand"]
async fn scan_finds_the_launchers_on_this_machine() {
    let scans = tokio::task::spawn_blocking(scan_all).await.expect("the scan runs");
    #[cfg(windows)]
    println!(
        "uninstall entries for Modrinth: {:?}",
        registry::install_locations(Launcher::Modrinth.display_names())
    );
    for scan in scans {
        println!("{}: folder {:?}, error {:?}", scan.launcher.name(), scan.folder, scan.error);
        for instance in &scan.instances {
            println!("  {} | {} | {:?}", instance.name, instance.source, instance.summary);
        }
    }
}

/// Prepares every importable instance on this machine and prints what each copy would take. Nothing is written.
#[tokio::test(flavor = "multi_thread")]
#[ignore = "walks this machine's instance folders; run by hand"]
async fn preview_the_copies_on_this_machine() {
    let scans = tokio::task::spawn_blocking(scan_all).await.expect("the scan runs");
    let picks: Vec<Pick> = scans
        .iter()
        .flat_map(|scan| {
            scan.instances.iter().filter(|instance| instance.summary.is_ok()).map(|instance| Pick {
                launcher: instance.launcher,
                source: instance.source.clone(),
            })
        })
        .collect();
    let paths = LauncherPaths::new(std::env::temp_dir().join("lodestone-preview"));
    let prepared = tokio::task::spawn_blocking(move || prepare(&picks, &paths, &[]))
        .await
        .expect("the preview runs")
        .expect("the instances prepare");
    for one in prepared {
        let payload = one.request.modpack.expect("a launcher import carries its folder");
        let origin = &payload.unpacks[0].origin;
        println!("{} -> {} bytes, {:?}", one.request.name, one.record.bytes, origin);
    }
}
