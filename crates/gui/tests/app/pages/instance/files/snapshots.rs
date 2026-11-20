use super::*;
use chrono::NaiveDate;
use lodestone_snapshots::model::Entry;
use lodestone_snapshots::{Kind, Snapshot, store};
use std::path::PathBuf;

fn scratch(name: &str) -> PathBuf {
    let dir = std::env::temp_dir().join(format!("lodestone-files-snapshots-{name}-{}", std::process::id()));
    let _ = std::fs::remove_dir_all(&dir);
    std::fs::create_dir_all(&dir).expect("the scratch folder can be made");
    dir
}

#[test]
fn an_instances_snapshots_are_read_newest_first_and_a_damaged_one_is_left_out() {
    let folder = scratch("scan");
    let dir = store::dir(&folder);
    let when = |second: u32| {
        NaiveDate::from_ymd_opt(2026, 11, 20)
            .expect("a date")
            .and_hms_opt(14, 5, second)
            .expect("a time")
    };
    let snapshot = Snapshot {
        minecraft: "1.21.1".to_string(),
        loader: "fabric".to_string(),
        modrinth: vec![Entry {
            modid: "AANobbMI".to_string(),
            version: Some("0.6.0".to_string()),
            fileid: "Yrmqwfh6".to_string(),
            name: "Sodium".to_string(),
        }],
        ..Snapshot::default()
    };
    store::write(&dir, Kind::UpdateAll, when(1), snapshot.clone(), &[]).expect("the first snapshot is written");
    store::write(&dir, Kind::Manual, when(9), snapshot.clone(), &[]).expect("the second snapshot is written");
    std::fs::write(dir.join("2026-11-20_14-05-05_manual.json.xz"), b"not an xz stream").expect("the damaged file is written");

    let found = scan(&folder);
    assert_eq!(found.len(), 2, "the damaged snapshot is left out");
    assert_eq!(found[0].listed.name.kind, Kind::Manual);
    assert_eq!(found[0].snapshot, snapshot);
    assert_eq!(found[1].listed.name.kind, Kind::UpdateAll);
}

#[test]
fn an_instance_without_a_snapshots_folder_has_none() {
    assert!(scan(&scratch("scan-none")).is_empty());
}
