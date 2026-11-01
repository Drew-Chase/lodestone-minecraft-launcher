use super::*;
use crate::app::shared_library::ModSource;

#[test]
fn only_the_archives_directly_in_the_resource_folders_are_linkable() {
    assert!(is_linkable("mods/sodium.jar"));
    assert!(is_linkable("resourcepacks/faithful.zip"));
    assert!(is_linkable("shaderpacks/complementary.zip"));
    assert!(is_linkable("datapacks/pack.zip"));
    assert!(!is_linkable("mods/nested/sodium.jar"), "a file in a subfolder is not identified");
    assert!(!is_linkable("mods/sodium.jar.disabled"));
    assert!(!is_linkable("config/sodium.jar"));
    assert!(!is_linkable("mods"));
}

#[test]
fn a_name_is_made_safe_to_be_a_file_name() {
    assert_eq!(file_safe("Enhanced: Worlds/Pack"), "Enhanced_ Worlds_Pack");
    assert_eq!(file_safe("   "), "export");
    assert_eq!(file_safe("Plain Name 1.0"), "Plain Name 1.0");
}

#[test]
fn the_partial_file_sits_beside_the_target() {
    assert_eq!(
        partial_path(Path::new("/exports/pack.mrpack")),
        PathBuf::from("/exports/pack.mrpack.partial")
    );
}

fn mod_record(platform: &str, file_name: &str, sha1: &str) -> ModSource {
    ModSource {
        instance: "instance-id".to_string(),
        platform: platform.to_string(),
        project_id: "238222".to_string(),
        version_id: "4123456".to_string(),
        name: "Sodium".to_string(),
        file_name: file_name.to_string(),
        sha1: Some(sha1.to_string()),
    }
}

#[test]
fn a_curseforge_link_needs_a_recorded_file_whose_hash_matches() {
    let records = vec![mod_record("curseforge", "sodium.jar", "ABC")];
    let link = curseforge_link(&records, "mods/sodium.jar", "abc").expect("linked");
    assert_eq!(link.project, "238222");
    assert_eq!(link.file, "4123456");
    assert!(
        curseforge_link(&records, "mods/sodium.jar", "def").is_none(),
        "a changed jar is not linked"
    );
    assert!(
        curseforge_link(&records, "config/sodium.jar", "abc").is_none(),
        "only mods are linked this way"
    );
    let modrinth = vec![mod_record("modrinth", "sodium.jar", "abc")];
    assert!(
        curseforge_link(&modrinth, "mods/sodium.jar", "abc").is_none(),
        "a Modrinth record is not a CurseForge link"
    );
}

#[test]
fn each_entry_takes_the_digests_and_links_found_for_its_path() {
    let planned = vec![
        Planned {
            path: "config/options.toml".to_string(),
            source: PathBuf::from("/a"),
            size: 3,
        },
        Planned {
            path: "mods/sodium.jar".to_string(),
            source: PathBuf::from("/b"),
            size: 9,
        },
    ];
    let digests = Digests {
        sha1: "1".repeat(40),
        sha512: "2".repeat(128),
    };
    let hashed = vec![Hashed {
        path: "mods/sodium.jar".to_string(),
        digests: digests.clone(),
    }];
    let mut links = HashMap::new();
    links.insert(
        "mods/sodium.jar".to_string(),
        Links {
            curseforge: Some(CurseForgeLink {
                project: "1".to_string(),
                file: "2".to_string(),
            }),
            ..Links::default()
        },
    );
    let entries = entries(planned, hashed, links);
    assert!(entries[0].digests.is_none());
    assert!(entries[0].links.is_empty());
    assert_eq!(entries[1].digests, Some(digests));
    assert!(entries[1].links.curseforge.is_some());
    assert_eq!(entries[1].size, 9);
}

#[test]
fn a_stage_reports_a_fraction_only_when_it_knows_its_total() {
    let progress = ExportProgress {
        stage: Stage::Hashing,
        done: 1,
        total: 4,
    };
    assert_eq!(progress.fraction(), Some(0.25));
    let unknown = ExportProgress {
        stage: Stage::Identifying,
        done: 0,
        total: 0,
    };
    assert_eq!(unknown.fraction(), None);
}
