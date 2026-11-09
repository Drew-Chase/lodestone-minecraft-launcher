use std::cell::Cell;

use super::*;

/// A map of files to the hashes they carry, where `None` stands for a hash that is not known.
fn files(pairs: &[(&str, Option<&str>)]) -> BTreeMap<String, Option<String>> {
    pairs.iter().map(|(path, hash)| (path.to_string(), hash.map(str::to_string))).collect()
}

/// The paths the instance holds.
fn held(paths: &[&str]) -> BTreeSet<String> {
    paths.iter().map(|path| path.to_string()).collect()
}

/// The hash of each path the instance holds. A path missing here cannot be read.
fn disk(pairs: &[(&str, &str)]) -> BTreeMap<String, String> {
    pairs.iter().map(|(path, hash)| (path.to_string(), hash.to_string())).collect()
}

/// The changes `input` makes, with the instance's files read from `on_disk`.
fn planned(input: &Input, on_disk: &[(&str, &str)]) -> Vec<Change> {
    let table = disk(on_disk);
    let mut hash = |path: &str| table.get(path).cloned();
    plan(input, &mut hash)
}

/// The kind of the change to `path`, when there is one.
fn kind_of(changes: &[Change], path: &str) -> Option<Kind> {
    changes.iter().find(|change| change.path == path).map(|change| change.kind.clone())
}

#[test]
fn a_file_the_release_adds_is_added() {
    let input = Input {
        release: files(&[("config/a.toml", Some("n"))]),
        ..Input::default()
    };
    assert_eq!(kind_of(&planned(&input, &[]), "config/a.toml"), Some(Kind::Add));
}

#[test]
fn a_config_the_user_deleted_that_the_release_still_ships_is_restored() {
    let input = Input {
        installed: files(&[("config/a.toml", Some("o"))]),
        release: files(&[("config/a.toml", Some("o"))]),
        ..Input::default()
    };
    assert_eq!(kind_of(&planned(&input, &[]), "config/a.toml"), Some(Kind::Restore));
}

#[test]
fn a_mod_the_user_deleted_is_restored_and_taken_by_default() {
    let input = Input {
        installed: files(&[("mods/m.jar", Some("o"))]),
        release: files(&[("mods/m.jar", Some("o"))]),
        ..Input::default()
    };
    let changes = planned(&input, &[]);
    assert_eq!(kind_of(&changes, "mods/m.jar"), Some(Kind::Restore));
    assert_eq!(changes[0].default_decision(), Decision::Take);
}

#[test]
fn a_file_only_the_release_changed_is_updated() {
    let input = Input {
        installed: files(&[("config/a.toml", Some("o"))]),
        release: files(&[("config/a.toml", Some("n"))]),
        held: held(&["config/a.toml"]),
        ..Input::default()
    };
    assert_eq!(kind_of(&planned(&input, &[("config/a.toml", "o")]), "config/a.toml"), Some(Kind::Update));
}

#[test]
fn a_file_both_sides_changed_is_a_conflict_the_user_keeps_by_default() {
    let input = Input {
        installed: files(&[("config/a.toml", Some("o"))]),
        release: files(&[("config/a.toml", Some("n"))]),
        held: held(&["config/a.toml"]),
        ..Input::default()
    };
    let changes = planned(&input, &[("config/a.toml", "x")]);
    assert_eq!(kind_of(&changes, "config/a.toml"), Some(Kind::Conflict));
    assert_eq!(changes[0].default_decision(), Decision::Keep);
}

#[test]
fn a_file_already_at_the_release_hash_needs_nothing() {
    let input = Input {
        installed: files(&[("config/a.toml", Some("o"))]),
        release: files(&[("config/a.toml", Some("n"))]),
        held: held(&["config/a.toml"]),
        ..Input::default()
    };
    assert!(planned(&input, &[("config/a.toml", "n")]).is_empty());
}

#[test]
fn a_config_only_the_user_changed_is_theirs_and_kept() {
    let input = Input {
        installed: files(&[("config/a.toml", Some("o"))]),
        release: files(&[("config/a.toml", Some("o"))]),
        held: held(&["config/a.toml"]),
        ..Input::default()
    };
    let changes = planned(&input, &[("config/a.toml", "x")]);
    assert_eq!(kind_of(&changes, "config/a.toml"), Some(Kind::Mine));
    assert_eq!(changes[0].default_decision(), Decision::Keep);
}

#[test]
fn an_unchanged_config_the_user_did_not_touch_needs_nothing() {
    let input = Input {
        installed: files(&[("config/a.toml", Some("o"))]),
        release: files(&[("config/a.toml", Some("o"))]),
        held: held(&["config/a.toml"]),
        ..Input::default()
    };
    assert!(planned(&input, &[("config/a.toml", "o")]).is_empty());
}

#[test]
fn a_mod_the_release_does_not_change_is_left_alone_without_being_hashed() {
    let input = Input {
        installed: files(&[("mods/m.jar", Some("o"))]),
        release: files(&[("mods/m.jar", Some("o"))]),
        held: held(&["mods/m.jar"]),
        ..Input::default()
    };
    let calls = Cell::new(0);
    let mut hash = |_: &str| {
        calls.set(calls.get() + 1);
        Some("x".to_string())
    };
    assert!(plan(&input, &mut hash).is_empty());
    assert_eq!(calls.get(), 0, "an unchanged jar is never read");
}

#[test]
fn a_mod_the_release_drops_is_removed_by_default() {
    let input = Input {
        installed: files(&[("mods/old.jar", Some("o"))]),
        held: held(&["mods/old.jar"]),
        ..Input::default()
    };
    let changes = planned(&input, &[("mods/old.jar", "o")]);
    assert_eq!(kind_of(&changes, "mods/old.jar"), Some(Kind::Drop));
    assert_eq!(changes[0].default_decision(), Decision::Take);
}

#[test]
fn a_config_the_release_drops_is_kept_by_default() {
    let input = Input {
        installed: files(&[("config/old.toml", Some("o"))]),
        held: held(&["config/old.toml"]),
        ..Input::default()
    };
    let changes = planned(&input, &[("config/old.toml", "o")]);
    assert_eq!(kind_of(&changes, "config/old.toml"), Some(Kind::Drop));
    assert_eq!(changes[0].default_decision(), Decision::Keep);
}

#[test]
fn with_no_installed_release_a_config_that_differs_is_a_conflict_and_one_that_matches_needs_nothing() {
    let input = Input {
        release: files(&[("config/a.toml", Some("n"))]),
        held: held(&["config/a.toml"]),
        ..Input::default()
    };
    assert_eq!(
        kind_of(&planned(&input, &[("config/a.toml", "x")]), "config/a.toml"),
        Some(Kind::Conflict)
    );
    assert!(planned(&input, &[("config/a.toml", "n")]).is_empty());
}

#[test]
fn a_jar_of_the_same_project_under_another_name_is_superseded_and_its_old_jar_is_not_dropped() {
    let input = Input {
        installed: files(&[("mods/a-1.jar", Some("h1"))]),
        release: files(&[("mods/a-2.jar", Some("h2"))]),
        held: held(&["mods/a-1.jar"]),
        jar_projects: BTreeMap::from([("mods/a-1.jar".to_string(), "modrinth/P".to_string())]),
        release_projects: BTreeMap::from([("mods/a-2.jar".to_string(), "modrinth/P".to_string())]),
    };
    let changes = planned(&input, &[("mods/a-1.jar", "h1")]);
    assert_eq!(
        changes,
        vec![Change {
            path: "mods/a-2.jar".to_string(),
            kind: Kind::Supersede {
                removes: "mods/a-1.jar".to_string(),
            },
        }]
    );
    assert_eq!(changes[0].default_decision(), Decision::Take);
}

#[test]
fn a_mod_the_user_disabled_stays_disabled() {
    let input = Input {
        installed: files(&[("mods/x.jar", Some("h"))]),
        release: files(&[("mods/x.jar", Some("h"))]),
        held: held(&["mods/x.jar.disabled"]),
        ..Input::default()
    };
    assert_eq!(kind_of(&planned(&input, &[]), "mods/x.jar"), None);
}

#[test]
fn a_jar_the_user_added_is_yours_and_stays() {
    let input = Input {
        held: held(&["mods/mine.jar"]),
        ..Input::default()
    };
    let changes = planned(&input, &[]);
    assert_eq!(kind_of(&changes, "mods/mine.jar"), Some(Kind::Yours));
    assert_eq!(changes[0].default_decision(), Decision::Keep);
}

#[test]
fn a_resource_pack_the_release_ships_is_added_only_where_it_is_missing() {
    let missing = Input {
        release: files(&[("resourcepacks/P.zip", Some("h"))]),
        ..Input::default()
    };
    assert_eq!(kind_of(&planned(&missing, &[]), "resourcepacks/P.zip"), Some(Kind::Add));
    let present = Input {
        release: files(&[("resourcepacks/P.zip", Some("h"))]),
        held: held(&["resourcepacks/P.zip"]),
        ..Input::default()
    };
    assert!(planned(&present, &[("resourcepacks/P.zip", "x")]).is_empty());
}

#[test]
fn the_options_file_is_added_only_where_it_is_missing_and_never_replaced() {
    let missing = Input {
        release: files(&[("options.txt", Some("h"))]),
        ..Input::default()
    };
    assert_eq!(kind_of(&planned(&missing, &[]), "options.txt"), Some(Kind::Add));
    let present = Input {
        installed: files(&[("options.txt", Some("o"))]),
        release: files(&[("options.txt", Some("n"))]),
        held: held(&["options.txt"]),
        ..Input::default()
    };
    assert!(planned(&present, &[("options.txt", "x")]).is_empty());
}
