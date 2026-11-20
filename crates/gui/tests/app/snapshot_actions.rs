use super::*;
use crate::app::pages::instance::files::ModFile;
use crate::app::pages::instance::mods::Row;
use crate::app::shared_library::ModSource;

fn file(name: &str, version: Option<&str>) -> ModFile {
    ModFile {
        file_name: name.to_string(),
        enabled: true,
        name: name.to_string(),
        author: None,
        version: version.map(str::to_string),
        id: None,
        dependencies: Vec::new(),
        icon: None,
    }
}

fn record(platform: &str, project: &str, version_id: &str, number: Option<&str>) -> ModSource {
    ModSource {
        instance: "inst".to_string(),
        platform: platform.to_string(),
        project_id: project.to_string(),
        version_id: version_id.to_string(),
        version_number: number.map(str::to_string),
        name: format!("Mod {project}"),
        file_name: "unused.jar".to_string(),
        sha1: None,
    }
}

fn row(name: &str, source: Option<ModSource>) -> Row {
    Row {
        file: file(name, Some("1.0")),
        source,
        update: None,
        library: false,
    }
}

#[test]
fn linked_mods_go_to_their_platform_and_the_rest_are_local_files() {
    let rows = [
        row("sodium.jar", Some(record("modrinth", "AANobbMI", "Yrmqwfh6", Some("0.6.0")))),
        row("fabric-api.jar", Some(record("curseforge", "306612", "49860", Some("0.92")))),
        row("custom.jar", None),
    ];
    let refs: Vec<&Row> = rows.iter().collect();
    let mods = PathBuf::from("/inst/mods");
    let (snapshot, local) = gather("1.21.1", "fabric".to_string(), &mods, &refs);

    assert_eq!(snapshot.minecraft, "1.21.1");
    assert_eq!(snapshot.loader, "fabric");
    assert_eq!(snapshot.modrinth.len(), 1);
    assert_eq!(snapshot.modrinth[0].modid, "AANobbMI");
    assert_eq!(snapshot.modrinth[0].fileid, "Yrmqwfh6");
    assert_eq!(snapshot.modrinth[0].version.as_deref(), Some("0.6.0"));
    assert_eq!(snapshot.curseforge.len(), 1);
    assert_eq!(snapshot.curseforge[0].fileid, "49860");
    assert_eq!(
        local,
        vec![LocalFile {
            file: "custom.jar".to_string(),
            path: mods.join("custom.jar")
        }]
    );
}

#[test]
fn a_mod_with_no_recorded_number_takes_the_version_its_jar_declares() {
    let rows = [row("lithium.jar", Some(record("modrinth", "P7dR8mSH", "v2", None)))];
    let refs: Vec<&Row> = rows.iter().collect();
    let (snapshot, _) = gather("1.21.1", "fabric".to_string(), Path::new("/inst/mods"), &refs);
    assert_eq!(snapshot.modrinth[0].version.as_deref(), Some("1.0"));
}

#[test]
fn a_record_on_a_platform_this_build_does_not_know_is_kept_as_a_local_jar() {
    let rows = [row("odd.jar", Some(record("other", "1", "2", None)))];
    let refs: Vec<&Row> = rows.iter().collect();
    let (snapshot, local) = gather("1.21.1", "fabric".to_string(), Path::new("/inst/mods"), &refs);
    assert_eq!(snapshot.linked_count(), 0);
    assert_eq!(local.len(), 1);
}

#[test]
fn a_record_names_the_number_it_has_or_else_the_one_its_jar_declares() {
    let declared = HashMap::from([("sodium.jar".to_string(), Some("0.6.0".to_string()))]);
    let mut named = record("modrinth", "AANobbMI", "Yrmqwfh6", Some("0.6.1"));
    named.file_name = "sodium.jar".to_string();
    assert_eq!(
        installed_of(&named, &declared).and_then(|installed| installed.version).as_deref(),
        Some("0.6.1")
    );

    let mut unnamed = record("modrinth", "AANobbMI", "Yrmqwfh6", None);
    unnamed.file_name = "sodium.jar".to_string();
    assert_eq!(
        installed_of(&unnamed, &declared).and_then(|installed| installed.version).as_deref(),
        Some("0.6.0")
    );

    let unknown = record("other", "1", "2", None);
    assert!(
        installed_of(&unknown, &declared).is_none(),
        "a platform this build does not know is not planned"
    );
}

#[test]
fn a_local_jar_is_hashed_only_when_mods_holds_it() {
    let root = std::env::temp_dir().join(format!("lodestone-snapshot-actions-hash-{}", std::process::id()));
    std::fs::create_dir_all(&root).expect("the scratch folder can be made");
    std::fs::write(root.join("custom.jar"), b"abc").expect("the jar is written");
    let local = vec![
        LocalJar {
            file: "custom.jar".to_string(),
            sha1: "unused".to_string(),
        },
        LocalJar {
            file: "gone.jar".to_string(),
            sha1: "unused".to_string(),
        },
    ];
    let present = hash_present(&root, &local);
    assert_eq!(present.len(), 1);
    assert_eq!(present["custom.jar"], "a9993e364706816aba3e25717850c26c9cd0d89d");
    let _ = std::fs::remove_dir_all(&root);
}

#[test]
fn a_loader_that_is_not_named_reads_as_vanilla() {
    assert_eq!(loader_label(""), "Vanilla");
    assert_eq!(loader_label("fabric"), "fabric");
}

#[test]
fn a_full_rollback_picks_only_the_mods_that_are_not_at_the_snapshot_release() {
    let snapshot = Snapshot {
        minecraft: "1.21.1".to_string(),
        loader: "fabric".to_string(),
        modrinth: vec![
            Entry {
                modid: "AANobbMI".to_string(),
                version: Some("0.6.0".to_string()),
                fileid: "v1".to_string(),
                name: "Sodium".to_string(),
            },
            Entry {
                modid: "P7dR8mSH".to_string(),
                version: None,
                fileid: "v2".to_string(),
                name: "Lithium".to_string(),
            },
        ],
        ..Snapshot::default()
    };
    let target = Target {
        minecraft: "1.21.1".to_string(),
        loader: "fabric".to_string(),
    };
    let installed = vec![Installed {
        platform: Platform::Modrinth,
        project_id: "AANobbMI".to_string(),
        version_id: "v1".to_string(),
        version: None,
        file_name: "sodium.jar".to_string(),
    }];
    let plan = plan::plan(&snapshot, &target, &installed, &HashMap::new()).expect("the snapshot plans");
    let prepared = Prepared {
        stem: "2026-11-20_14-05-33_manual".to_string(),
        when: chrono::NaiveDate::from_ymd_opt(2026, 11, 20)
            .expect("a date")
            .and_hms_opt(14, 5, 33)
            .expect("a time"),
        plan,
        zip: None,
    };
    assert_eq!(
        changes_of(&prepared),
        vec![1],
        "Sodium is at the snapshot's release, and Lithium is not installed"
    );
}
