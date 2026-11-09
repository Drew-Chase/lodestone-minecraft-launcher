use std::path::PathBuf;

use super::*;

/// A scratch data root unique to this run and `name`, emptied first.
fn data(name: &str) -> PathBuf {
    let dir = std::env::temp_dir().join(format!("lodestone-journal-{name}-{}", std::process::id()));
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

/// The workspace of the update of the instance `id` under `data`, and the instance's folder.
fn workspace_of(data: &Path, id: &str) -> (Workspace, PathBuf) {
    let paths = LauncherPaths::new(data);
    let instance = paths.instance(id);
    (Workspace::new(paths.pack_journal(id), instance.clone(), paths.pack_base(id)), instance)
}

#[test]
fn rolling_back_restores_a_moved_file_and_removes_a_created_one() {
    let data = data("roll");
    let (ws, instance) = workspace_of(&data, "inst");
    put(&instance.join("mods/old.jar"), "old");
    ws.create().expect("the workspace is made");
    ws.write_journal(&Journal {
        token: "t".to_string(),
        moved: vec!["mods/old.jar".to_string()],
        created: vec!["mods/added.jar".to_string()],
    })
    .expect("the journal is written");
    // What the update did: it moved the original aside, then wrote the release's copy and a new jar.
    move_file(&instance.join("mods/old.jar"), &ws.moved().join("mods/old.jar")).expect("moved");
    put(&instance.join("mods/old.jar"), "new");
    put(&instance.join("mods/added.jar"), "added");

    ws.roll_back().expect("rolled back");

    assert_eq!(read(&instance.join("mods/old.jar")), "old", "the original is back");
    assert!(!instance.join("mods/added.jar").exists(), "the created jar is gone");
    assert!(!ws.staging().exists(), "the workspace is removed");
}

#[test]
fn rolling_back_with_no_journal_only_removes_the_workspace() {
    let data = data("nojournal");
    let (ws, instance) = workspace_of(&data, "inst");
    put(&instance.join("config/a.toml"), "kept");
    ws.create().expect("the workspace is made");
    put(&ws.staging().join("config/a.toml"), "staged");

    ws.roll_back().expect("rolled back");

    assert_eq!(read(&instance.join("config/a.toml")), "kept");
    assert!(!ws.staging().exists());
}

#[test]
fn a_journal_entry_the_update_never_reached_is_skipped() {
    let data = data("unreached");
    let (ws, instance) = workspace_of(&data, "inst");
    put(&instance.join("mods/x.jar"), "x");
    ws.create().expect("the workspace is made");
    ws.write_journal(&Journal {
        token: "t".to_string(),
        moved: vec!["mods/x.jar".to_string()],
        created: vec!["mods/y.jar".to_string()],
    })
    .expect("the journal is written");

    ws.roll_back().expect("rolled back without error");

    assert_eq!(read(&instance.join("mods/x.jar")), "x", "a jar that was never moved stays");
}

#[test]
fn finishing_swaps_in_the_new_pristine_copies_and_discards_the_originals() {
    let data = data("finish");
    let (ws, instance) = workspace_of(&data, "inst");
    put(&ws.base().join("config/a.toml"), "old base");
    ws.create().expect("the workspace is made");
    put(&ws.base_new().join("config/a.toml"), "new base");
    put(&ws.moved().join("mods/old.jar"), "old jar");
    ws.write_journal(&Journal {
        token: "t".to_string(),
        moved: vec!["mods/old.jar".to_string()],
        created: Vec::new(),
    })
    .expect("the journal is written");

    ws.finish().expect("finished");

    assert_eq!(read(&ws.base().join("config/a.toml")), "new base");
    assert!(!ws.root.exists(), "the workspace is removed");
    assert!(!instance.join("mods/old.jar").exists());
}

#[test]
fn settling_finishes_a_committed_update_and_rolls_back_the_rest() {
    let data = data("settle");
    let paths = LauncherPaths::new(&data);
    let (committed_ws, committed_instance) = workspace_of(&data, "committed");
    put(&committed_ws.base().join("config/a.toml"), "old");
    committed_ws.create().expect("made");
    put(&committed_ws.base_new().join("config/a.toml"), "new");
    committed_ws
        .write_journal(&Journal {
            token: "tok-a".to_string(),
            moved: Vec::new(),
            created: Vec::new(),
        })
        .expect("journal");

    let (stopped_ws, stopped_instance) = workspace_of(&data, "stopped");
    put(&stopped_instance.join("mods/x.jar"), "x");
    stopped_ws.create().expect("made");
    stopped_ws
        .write_journal(&Journal {
            token: "tok-b".to_string(),
            moved: vec!["mods/x.jar".to_string()],
            created: Vec::new(),
        })
        .expect("journal");
    move_file(&stopped_instance.join("mods/x.jar"), &stopped_ws.moved().join("mods/x.jar")).expect("moved");

    let (gone_ws, _) = workspace_of(&data, "gone");
    gone_ws.create().expect("made");

    let (empty_ws, empty_instance) = workspace_of(&data, "empty");
    empty_ws.create().expect("made");
    put(&empty_instance.join("config/keep.toml"), "keep");

    let instances = BTreeMap::from([
        ("committed".to_string(), committed_instance.clone()),
        ("stopped".to_string(), stopped_instance.clone()),
        ("empty".to_string(), empty_instance.clone()),
    ]);
    let committed = BTreeMap::from([("committed".to_string(), "tok-a".to_string())]);

    let rolled_back = settle(&paths, &instances, &committed);

    assert_eq!(rolled_back, vec!["stopped".to_string()]);
    assert_eq!(
        read(&committed_ws.base().join("config/a.toml")),
        "new",
        "the committed update's copies take their place"
    );
    assert_eq!(read(&stopped_instance.join("mods/x.jar")), "x", "the stopped update's jar is back");
    assert!(!gone_ws.root.exists(), "a workspace of an instance that is gone is removed");
    assert!(!empty_ws.root.exists(), "a workspace with no journal is removed");
    assert_eq!(
        read(&empty_instance.join("config/keep.toml")),
        "keep",
        "nothing of an instance with no journal is touched"
    );
}
