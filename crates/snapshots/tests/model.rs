use super::*;

#[test]
fn the_limit_is_held_between_zero_and_twenty() {
    assert_eq!(clamp_limit(0), 0);
    assert_eq!(clamp_limit(5), 5);
    assert_eq!(clamp_limit(20), 20);
    assert_eq!(clamp_limit(21), 20);
    assert_eq!(clamp_limit(u32::MAX), 20);
    assert_eq!(LIMIT_DEFAULT, 5);
}

#[test]
fn platforms_have_the_keys_the_json_and_the_records_use() {
    assert_eq!(Platform::Modrinth.key(), "modrinth");
    assert_eq!(Platform::CurseForge.key(), "curseforge");
    assert_eq!(Platform::from_key("curseforge"), Some(Platform::CurseForge));
    assert_eq!(Platform::from_key("other"), None);
}

#[test]
fn the_json_keeps_the_keys_the_format_names() {
    let snapshot = Snapshot {
        minecraft: "1.21.1".into(),
        loader: "fabric".into(),
        modrinth: vec![Entry {
            modid: "AANobbMI".into(),
            version: Some("mc1.21.1-0.6.0".into()),
            fileid: "Yrmqwfh6".into(),
            name: "Sodium".into(),
        }],
        curseforge: vec![Entry {
            modid: "306612".into(),
            version: None,
            fileid: "49860".into(),
            name: "Fabric API".into(),
        }],
        local: vec![LocalJar {
            file: "custom.jar".into(),
            sha1: "abc".into(),
        }],
    };
    let value = serde_json::to_value(&snapshot).expect("the snapshot serializes");
    assert_eq!(value["minecraft"], "1.21.1");
    assert_eq!(value["loader"], "fabric");
    assert_eq!(value["modrinth"][0]["modid"], "AANobbMI");
    assert_eq!(value["modrinth"][0]["version"], "mc1.21.1-0.6.0");
    assert_eq!(value["modrinth"][0]["fileid"], "Yrmqwfh6");
    assert_eq!(value["modrinth"][0]["name"], "Sodium");
    assert!(value["curseforge"][0]["version"].is_null(), "a missing version number is written as null");
    assert_eq!(value["local"][0]["file"], "custom.jar");
    assert_eq!(value["local"][0]["sha1"], "abc");
}

#[test]
fn a_missing_section_reads_as_empty_and_unknown_keys_are_ignored() {
    let json = r#"{"minecraft":"1.21.1","loader":"fabric","modrinth":[],"future":{"x":1}}"#;
    let snapshot: Snapshot = serde_json::from_str(json).expect("the snapshot reads");
    assert_eq!(snapshot.minecraft, "1.21.1");
    assert!(snapshot.curseforge.is_empty());
    assert!(snapshot.local.is_empty());
}

#[test]
fn linked_entries_list_modrinth_first() {
    let snapshot = Snapshot {
        modrinth: vec![Entry {
            modid: "m".into(),
            version: None,
            fileid: "1".into(),
            name: "M".into(),
        }],
        curseforge: vec![Entry {
            modid: "c".into(),
            version: None,
            fileid: "2".into(),
            name: "C".into(),
        }],
        ..Snapshot::default()
    };
    let order: Vec<(Platform, &str)> = snapshot.linked().map(|(platform, entry)| (platform, entry.modid.as_str())).collect();
    assert_eq!(order, vec![(Platform::Modrinth, "m"), (Platform::CurseForge, "c")]);
    assert_eq!(snapshot.linked_count(), 2);
}

#[test]
fn kinds_have_the_names_the_file_names_use() {
    assert_eq!(Kind::UpdateAll.slug(), "update-all");
    assert_eq!(Kind::Manual.slug(), "manual");
    assert_eq!(Kind::Rollback.slug(), "rollback");
}
