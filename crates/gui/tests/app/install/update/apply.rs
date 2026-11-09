use std::path::PathBuf;

use super::*;
use crate::app::install::modpack::Payload;
use crate::app::install::pack::Release;
use crate::app::pages::library::instance::{Loader, fixture};
use crate::app::paths::LauncherPaths;

/// A scratch data root unique to this run and `name`, emptied first.
fn scratch(name: &str) -> PathBuf {
    let dir = std::env::temp_dir().join(format!("lodestone-apply-{name}-{}", std::process::id()));
    let _ = std::fs::remove_dir_all(&dir);
    std::fs::create_dir_all(&dir).expect("scratch data root");
    dir
}

/// Writes `text` to `path`, making its folders.
fn put(path: &Path, text: &str) {
    std::fs::create_dir_all(path.parent().expect("a parent folder")).expect("folders");
    std::fs::write(path, text).expect("file written");
}

/// The text of the file at `path`.
fn read(path: &Path) -> String {
    std::fs::read_to_string(path).expect("the file is there")
}

/// An update of the instance `id` in `paths`, onto release 2.0 of a Fabric pack, with no files of its own.
fn request(paths: &LauncherPaths, id: &str) -> UpdateRequest {
    UpdateRequest {
        paths: paths.clone(),
        instance: fixture(id, "Pack", Loader::Fabric),
        release: Release {
            id: "v2".to_string(),
            number: "2.0".to_string(),
        },
        mc_version: "1.20.1".to_string(),
        loader: Loader::Fabric,
        loader_version: Some("0.16.0".to_string()),
        payload: Payload {
            files: Vec::new(),
            unpacks: Vec::new(),
            sources: Vec::new(),
            packs: Vec::new(),
        },
        backup_worlds: false,
        decisions: Decisions::default(),
        record_only: false,
        reviewed: None,
    }
}

/// A change to `path` of `kind`, with `decision`.
fn change(path: &str, kind: Kind, decision: Decision) -> (Change, Decision) {
    (
        Change {
            path: path.to_string(),
            kind,
        },
        decision,
    )
}

#[test]
fn each_decision_moves_and_writes_what_it_says_and_nothing_else() {
    let data = scratch("operations");
    let instance = data.join("instance");
    let staging = data.join("staging");
    let old_base = data.join("base");
    put(&instance.join("mods/old.jar"), "old");
    put(&instance.join("config/held.toml"), "held");
    let changes = vec![
        change("config/new.toml", Kind::Add, Decision::Take),
        change("mods/old.jar", Kind::Drop, Decision::Take),
        change("config/held.toml", Kind::Drop, Decision::Keep),
        change("config/held.toml", Kind::Mine, Decision::Take),
        change("mods/kept.jar", Kind::Restore, Decision::Keep),
    ];

    let ops = operations(&changes, &instance, &staging, &old_base, &Decisions::default()).expect("planned");

    assert_eq!(ops.moved, vec!["config/held.toml".to_string(), "mods/old.jar".to_string()]);
    assert_eq!(ops.created, vec!["config/new.toml".to_string()]);
    let written: Vec<&str> = ops.writes.iter().map(|(path, _)| path.as_str()).collect();
    assert_eq!(written, vec!["config/new.toml", "config/held.toml"]);
}

#[test]
fn a_kept_conflict_takes_the_release_lines_that_do_not_touch_the_users_and_keeps_the_users_own() {
    let data = scratch("merge");
    let instance = data.join("instance");
    let staging = data.join("staging");
    let old_base = data.join("base");
    put(&old_base.join("config/a.toml"), "x=1\nmid=0\nkey=2\nend\n");
    put(&instance.join("config/a.toml"), "x=1\nmid=0\nkey=USER\nend\n");
    put(&staging.join("config/a.toml"), "x=RELEASE\nmid=0\nkey=RELEASE\nend\n");
    let changes = vec![change("config/a.toml", Kind::Conflict, Decision::Keep)];

    let ops = operations(&changes, &instance, &staging, &old_base, &Decisions::default()).expect("planned");

    let merged = ops
        .writes
        .iter()
        .find_map(|(path, write)| match write {
            Write::Merged(bytes) if path == "config/a.toml" => Some(bytes.clone()),
            _ => None,
        })
        .expect("the conflict is merged");
    assert_eq!(String::from_utf8(merged).expect("text"), "x=RELEASE\nmid=0\nkey=USER\nend\n");
}

#[test]
fn each_hunk_of_a_kept_conflict_takes_the_side_its_choice_names() {
    let data = scratch("hunks");
    let instance = data.join("instance");
    let staging = data.join("staging");
    let old_base = data.join("base");
    put(&old_base.join("config/a.toml"), "x=1\nmid=0\nkey=2\nend\n");
    put(&instance.join("config/a.toml"), "x=1\nmid=0\nkey=USER\nend\n");
    put(&staging.join("config/a.toml"), "x=RELEASE\nmid=0\nkey=RELEASE\nend\n");
    let changes = vec![change("config/a.toml", Kind::Conflict, Decision::Keep)];
    let mut decisions = Decisions::default();
    decisions.choose("config/a.toml", 0, Side::Theirs);

    let ops = operations(&changes, &instance, &staging, &old_base, &decisions).expect("planned");

    let merged = ops
        .writes
        .iter()
        .find_map(|(path, write)| match write {
            Write::Merged(bytes) if path == "config/a.toml" => Some(bytes.clone()),
            _ => None,
        })
        .expect("the conflict is merged");
    assert_eq!(String::from_utf8(merged).expect("text"), "x=RELEASE\nmid=0\nkey=RELEASE\nend\n");
}

#[test]
fn a_conflict_with_no_pristine_copy_or_a_binary_file_writes_nothing_when_kept() {
    let data = scratch("unmergeable");
    let instance = data.join("instance");
    let staging = data.join("staging");
    let old_base = data.join("base");
    put(&instance.join("config/no-copy.toml"), "mine\n");
    put(&staging.join("config/no-copy.toml"), "theirs\n");
    put(&instance.join("config/image.bin"), "\u{0}binary");
    put(&old_base.join("config/image.bin"), "\u{0}binary");
    put(&staging.join("config/image.bin"), "\u{0}other");
    let changes = vec![
        change("config/no-copy.toml", Kind::Conflict, Decision::Keep),
        change("config/image.bin", Kind::Conflict, Decision::Keep),
    ];

    let ops = operations(&changes, &instance, &staging, &old_base, &Decisions::default()).expect("planned");

    assert!(ops.writes.is_empty(), "neither file is rewritten");
    assert!(ops.moved.is_empty());
}

#[test]
fn merging_keeps_the_users_file_when_the_merge_would_not_change_it() {
    let data = scratch("nochange");
    let yours = data.join("yours.toml");
    let base = data.join("base.toml");
    let theirs = data.join("theirs.toml");
    put(&yours, "a=1\n");
    put(&base, "a=1\n");
    put(&theirs, "a=1\n");
    assert_eq!(merged(&yours, &base, &theirs, |_| Side::Yours).expect("merged"), None);
}

#[test]
fn walking_a_staging_folder_lists_nested_files_with_slashes_and_leaves_out_the_fetch_folder() {
    let data = scratch("walk");
    put(&data.join("mods/a.jar"), "a");
    put(&data.join("config/deep/x.toml"), "x");
    put(&data.join(".modpack/archive.zip"), "z");

    let mut found = walk(&data);
    found.sort();

    assert_eq!(found, vec!["config/deep/x.toml".to_string(), "mods/a.jar".to_string()]);
}

#[test]
fn file_names_lists_files_and_not_folders() {
    let data = scratch("names");
    put(&data.join("mods/a.jar"), "a");
    std::fs::create_dir_all(data.join("mods/sub")).expect("folder");
    let mut names = file_names(&data.join("mods"));
    names.sort();
    assert_eq!(names, vec!["a.jar".to_string()]);
    assert!(file_names(&data.join("missing")).is_empty());
}

#[tokio::test]
async fn applying_journals_every_path_before_it_moves_one_and_leaves_the_release_in_place() {
    let data = scratch("apply");
    let paths = LauncherPaths::new(&data);
    let update = request(&paths, "pack");
    let instance = paths.instance("pack");
    put(&instance.join("mods/old.jar"), "old");
    put(&instance.join("config/a.toml"), "mine");
    let ws = workspace_for(&update);
    ws.create().expect("the workspace is made");
    put(&ws.staging().join("mods/new.jar"), "new");
    put(&ws.staging().join("config/a.toml"), "theirs");
    let planned = Planned {
        changes: vec![
            change(
                "mods/new.jar",
                Kind::Supersede {
                    removes: "mods/old.jar".to_string(),
                },
                Decision::Take,
            ),
            change("config/a.toml", Kind::Update, Decision::Take),
        ],
        release: BTreeMap::from([("mods/new.jar".to_string(), Some("n".to_string())), ("config/a.toml".to_string(), None)]),
    };

    let applied = apply(&update, &ws, planned, "tok").await.expect("applied");

    let journal = ws.read_journal().expect("read").expect("journaled");
    assert_eq!(journal.token, "tok");
    assert_eq!(journal.moved, vec!["config/a.toml".to_string(), "mods/old.jar".to_string()]);
    assert_eq!(journal.created, vec!["mods/new.jar".to_string()]);
    assert_eq!(read(&instance.join("mods/new.jar")), "new");
    assert_eq!(read(&instance.join("config/a.toml")), "theirs");
    assert_eq!(read(&ws.moved().join("mods/old.jar")), "old", "the original jar waits in the workspace");
    assert_eq!(read(&ws.moved().join("config/a.toml")), "mine", "the user's file waits in the workspace");
    assert_eq!(applied.removed_jars, vec!["old.jar".to_string()]);
    assert_eq!(applied.mod_count, 1);
    assert!(
        applied
            .files
            .iter()
            .any(|file| file.path == "mods/new.jar" && file.sha1.as_deref() == Some("n"))
    );
    assert!(applied.files.iter().any(|file| file.path == "config/a.toml" && file.sha1.is_none()));
    ws.roll_back().expect("rolled back");
    assert_eq!(read(&instance.join("mods/old.jar")), "old", "a rollback puts the original jar back");
    assert_eq!(read(&instance.join("config/a.toml")), "mine", "and the user's file");
    assert!(!instance.join("mods/new.jar").exists(), "and removes the jar it wrote");
}

/// The workspace an update of `request` would use.
fn workspace_for(request: &UpdateRequest) -> Workspace {
    Workspace::new(
        request.paths.pack_journal(&request.instance.id),
        request.paths.instance(&request.instance.directory_name),
        request.paths.pack_base(&request.instance.id),
    )
}
