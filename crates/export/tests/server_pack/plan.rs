use super::*;
use crate::support::{put, scratch};

/// A detection for a jar in an instance's `mods` folder, with `side` as its detected side.
fn detection(side: Option<Side>) -> ModDetection {
    ModDetection {
        path: PathBuf::from("/instance/mods/mod.jar"),
        filename: "mod.jar".to_string(),
        side,
        evidence: String::new(),
        loader: None,
        mod_id: None,
        icon: None,
    }
}

#[test]
fn with_no_choice_a_client_only_mod_is_left_out() {
    assert!(!takes(&detection(Some(Side::ClientOnly)), None));
}

#[test]
fn with_no_choice_server_only_both_sides_and_unsure_mods_are_taken() {
    assert!(takes(&detection(Some(Side::ServerOnly)), None));
    assert!(takes(&detection(Some(Side::ClientAndServer)), None));
    assert!(takes(&detection(None), None));
}

#[test]
fn a_choice_overrides_the_detected_side() {
    assert!(takes(&detection(Some(Side::ClientOnly)), Some(Choice::Include)));
    assert!(!takes(&detection(Some(Side::ServerOnly)), Some(Choice::Exclude)));
    assert!(!takes(&detection(None), Some(Choice::Exclude)));
}

#[test]
fn a_choice_is_remembered_by_the_mod_id_or_else_by_the_jar_path() {
    let mut mod_jar = detection(None);
    assert_eq!(key(&mod_jar), "/instance/mods/mod.jar");
    mod_jar.mod_id = Some("sodium".to_string());
    assert_eq!(key(&mod_jar), "sodium");
}

#[test]
fn the_plan_takes_the_default_folders_and_only_the_jars_it_is_given() {
    let root = scratch("server-pack-plan");
    put(&root, "mods/keep.jar", b"keep");
    put(&root, "mods/skip.jar", b"skip");
    put(&root, "mods/off.jar.disabled", b"off");
    put(&root, "config/a.toml", b"a");
    put(&root, "logs/latest.log", b"log");
    put(&root, "saves/world/level.dat", b"save");
    let jars = [root.join("mods/keep.jar")];
    let files = plan(&root, &Selection::new_export(), &jars).expect("the plan builds");
    let paths: Vec<&str> = files.iter().map(|file| file.path.as_str()).collect();
    assert_eq!(paths, ["config/a.toml", "mods/keep.jar"]);
}

#[test]
fn an_unticked_folder_stays_out_and_mods_is_decided_by_the_jars_alone() {
    let root = scratch("server-pack-plan-unticked");
    put(&root, "mods/keep.jar", b"keep");
    put(&root, "config/a.toml", b"a");
    put(&root, "defaultconfigs/b.toml", b"b");
    let selection = Selection::from_excluded(["config".to_string()]);
    let files = plan(&root, &selection, &[]).expect("the plan builds");
    let paths: Vec<&str> = files.iter().map(|file| file.path.as_str()).collect();
    assert_eq!(paths, ["defaultconfigs/b.toml"]);
}

#[test]
fn the_planned_jar_keeps_its_source_and_size() {
    let root = scratch("server-pack-plan-size");
    put(&root, "mods/keep.jar", b"twelve bytes");
    let files = plan(&root, &Selection::new_export(), &[root.join("mods/keep.jar")]).expect("the plan builds");
    let jar = files.iter().find(|file| file.path == "mods/keep.jar").expect("the jar is planned");
    assert_eq!(jar.source, root.join("mods/keep.jar"));
    assert_eq!(jar.size, 12);
}
