use super::*;
use crate::hash::sha1_of_file;
use crate::model::{Entry, LocalJar};
use crate::support::{put, scratch};
use chrono::NaiveDate;
use std::fs::{self, File};
use std::io::Read;

fn when(second: u32) -> NaiveDateTime {
    NaiveDate::from_ymd_opt(2026, 11, 20)
        .expect("a date")
        .and_hms_opt(14, 5, second)
        .expect("a time")
}

fn sample() -> Snapshot {
    Snapshot {
        minecraft: "1.21.1".into(),
        loader: "fabric".into(),
        modrinth: vec![Entry {
            modid: "AANobbMI".into(),
            version: Some("0.6.0".into()),
            fileid: "Yrmqwfh6".into(),
            name: "Sodium".into(),
        }],
        curseforge: Vec::new(),
        local: Vec::new(),
    }
}

fn local(file: &str, path: PathBuf) -> LocalFile {
    LocalFile { file: file.into(), path }
}

#[test]
fn a_snapshot_without_local_jars_is_one_json_file_that_reads_back() {
    let root = scratch("store-json-only");
    let folder = dir(&root);
    let listed = write(&folder, Kind::UpdateAll, when(33), sample(), &[]).expect("the snapshot is written");
    assert!(listed.zip.is_none());
    assert_eq!(listed.json, folder.join("2026-11-20_14-05-33_update-all.json.xz"));
    assert!(listed.json.is_file());
    assert!(
        !folder.join("2026-11-20_14-05-33_update-all.json.xz.partial").exists(),
        "no partial file is left behind"
    );
    assert_eq!(read(&listed.json).expect("the snapshot reads"), sample());
}

#[test]
fn local_jars_go_in_a_zip_beside_the_json_and_are_listed_with_their_hashes() {
    let root = scratch("store-local");
    let jar = put(&root, "mods/custom.jar", b"jar bytes");
    let folder = dir(&root);
    let listed = write(&folder, Kind::Manual, when(33), sample(), &[local("custom.jar", jar.clone())]).expect("the snapshot is written");
    let zip_path = listed.zip.clone().expect("a zip is written");
    assert_eq!(zip_path, folder.join("2026-11-20_14-05-33_manual.zip"));
    let read_back = read(&listed.json).expect("the snapshot reads");
    assert_eq!(
        read_back.local,
        vec![LocalJar {
            file: "custom.jar".into(),
            sha1: sha1_of_file(&jar).expect("the jar hashes"),
        }]
    );
    let mut archive = zip::ZipArchive::new(File::open(&zip_path).expect("the zip opens")).expect("the zip reads");
    let mut bytes = Vec::new();
    archive
        .by_name("custom.jar")
        .expect("the jar is in the zip")
        .read_to_end(&mut bytes)
        .expect("the jar reads");
    assert_eq!(bytes, b"jar bytes");
}

#[test]
fn a_snapshot_lists_newest_first_and_pairs_each_zip_with_its_json() {
    let root = scratch("store-list");
    let jar = put(&root, "mods/custom.jar", b"x");
    let folder = dir(&root);
    write(&folder, Kind::UpdateAll, when(10), sample(), &[]).expect("the first snapshot is written");
    write(&folder, Kind::Manual, when(20), sample(), &[local("custom.jar", jar)]).expect("the second snapshot is written");
    let listed = list(&folder).expect("the folder lists");
    assert_eq!(listed.len(), 2);
    assert_eq!(listed[0].name.kind, Kind::Manual);
    assert!(listed[0].zip.is_some());
    assert_eq!(listed[1].name.kind, Kind::UpdateAll);
    assert!(listed[1].zip.is_none());
}

#[test]
fn a_folder_that_does_not_exist_lists_nothing() {
    let root = scratch("store-missing");
    assert!(list(&dir(&root)).expect("a missing folder lists").is_empty());
}

#[test]
fn partial_files_and_orphan_zips_are_not_listed() {
    let root = scratch("store-ignore");
    let folder = dir(&root);
    put(&folder, "2026-11-20_14-05-33_manual.json.xz.partial", b"half");
    put(&folder, "2026-11-20_14-05-34_manual.zip", b"orphan");
    put(&folder, "notes.txt", b"not a snapshot");
    assert!(list(&folder).expect("the folder lists").is_empty());
}

#[test]
fn a_second_snapshot_in_the_same_second_takes_the_next_copy() {
    let root = scratch("store-copies");
    let folder = dir(&root);
    let first = write(&folder, Kind::Manual, when(7), sample(), &[]).expect("the first is written");
    let second = write(&folder, Kind::Manual, when(7), sample(), &[]).expect("the second is written");
    assert_eq!(
        first.json.file_name().and_then(|name| name.to_str()),
        Some("2026-11-20_14-05-07_manual.json.xz")
    );
    assert_eq!(
        second.json.file_name().and_then(|name| name.to_str()),
        Some("2026-11-20_14-05-07_manual-2.json.xz")
    );
    assert!(first.json.is_file() && second.json.is_file());
    assert_eq!(list(&folder).expect("the folder lists")[0].name.copy, 2);
}

#[test]
fn pruning_keeps_the_newest_and_removes_the_rest_with_their_zips() {
    let root = scratch("store-prune");
    let jar = put(&root, "mods/custom.jar", b"x");
    let folder = dir(&root);
    for second in 0..4 {
        write(&folder, Kind::Manual, when(second), sample(), &[local("custom.jar", jar.clone())]).expect("a snapshot is written");
    }
    let removed = prune(&folder, 2).expect("the folder prunes");
    assert_eq!(removed.len(), 2);
    let kept = list(&folder).expect("the folder lists");
    assert_eq!(kept.len(), 2);
    assert_eq!(kept[0].name.when, when(3));
    assert_eq!(kept[1].name.when, when(2));
    assert_eq!(
        fs::read_dir(&folder).expect("the folder reads").count(),
        4,
        "two JSON files and their two zips"
    );
}

#[test]
fn pruning_to_zero_removes_every_snapshot() {
    let root = scratch("store-prune-zero");
    let folder = dir(&root);
    write(&folder, Kind::Manual, when(1), sample(), &[]).expect("a snapshot is written");
    assert_eq!(prune(&folder, 0).expect("the folder prunes").len(), 1);
    assert!(list(&folder).expect("the folder lists").is_empty());
}

#[test]
fn a_local_jar_is_restored_only_when_its_bytes_match_the_recorded_hash() {
    let root = scratch("store-restore");
    let jar = put(&root, "mods/custom.jar", b"jar bytes");
    let sha1 = sha1_of_file(&jar).expect("the jar hashes");
    let folder = dir(&root);
    let listed = write(&folder, Kind::Manual, when(1), sample(), &[local("custom.jar", jar)]).expect("the snapshot is written");
    let zip_path = listed.zip.expect("a zip is written");

    let dest = root.join("restored/custom.jar");
    fs::create_dir_all(dest.parent().expect("a parent")).expect("the folder can be made");
    restore_local(&zip_path, "custom.jar", &sha1, &dest).expect("the jar restores");
    assert_eq!(fs::read(&dest).expect("the restored jar reads"), b"jar bytes");
    assert!(!partial_of(&dest).exists());

    let other = root.join("restored/other.jar");
    let error = restore_local(&zip_path, "custom.jar", "0000", &other).expect_err("a wrong hash is refused");
    assert!(matches!(error, Error::Checksum { .. }));
    assert!(!other.exists() && !partial_of(&other).exists(), "nothing is left in place");

    let error = restore_local(&zip_path, "custom.jar", &sha1, &dest).expect_err("an existing file is kept");
    assert!(matches!(error, Error::Exists(_)));
}
