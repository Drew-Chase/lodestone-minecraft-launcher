use super::*;
use crate::model::{Entry, LocalJar, Platform, Snapshot};
use std::collections::HashMap;

fn snapshot() -> Snapshot {
    Snapshot {
        minecraft: "1.21.1".into(),
        loader: "fabric".into(),
        modrinth: vec![
            Entry {
                modid: "AANobbMI".into(),
                version: Some("0.6.0".into()),
                fileid: "v1".into(),
                name: "Sodium".into(),
            },
            Entry {
                modid: "P7dR8mSH".into(),
                version: None,
                fileid: "v2".into(),
                name: "Lithium".into(),
            },
            Entry {
                modid: "gone".into(),
                version: Some("1".into()),
                fileid: "v3".into(),
                name: "Gone Mod".into(),
            },
        ],
        curseforge: vec![Entry {
            modid: "306612".into(),
            version: Some("Fabric API 0.9".into()),
            fileid: "49860".into(),
            name: "Fabric API".into(),
        }],
        local: vec![LocalJar {
            file: "custom.jar".into(),
            sha1: "aaaa".into(),
        }],
    }
}

fn target() -> Target {
    Target {
        minecraft: "1.21.1".into(),
        loader: "fabric".into(),
    }
}

fn installed() -> Vec<Installed> {
    vec![
        Installed {
            platform: Platform::Modrinth,
            project_id: "AANobbMI".into(),
            version_id: "v9".into(),
            version: Some("0.7.0".into()),
            file_name: "sodium.jar".into(),
        },
        Installed {
            platform: Platform::Modrinth,
            project_id: "P7dR8mSH".into(),
            version_id: "v2".into(),
            version: None,
            file_name: "lithium.jar".into(),
        },
        Installed {
            platform: Platform::CurseForge,
            project_id: "306612".into(),
            version_id: "49860".into(),
            version: None,
            file_name: "fabric-api.jar".into(),
        },
    ]
}

#[test]
fn each_mod_is_the_same_changed_or_not_installed() {
    let outcome = plan(&snapshot(), &target(), &installed(), &HashMap::new()).expect("the snapshot plans");
    let states: Vec<State> = outcome.items.iter().map(|item| item.state).collect();
    assert_eq!(states, vec![State::Changed, State::Same, State::NotInstalled, State::Same]);
    assert_eq!(outcome.changes(), 2);
    assert_eq!(outcome.items[0].installed.as_ref().map(|record| record.version_id.as_str()), Some("v9"));
    assert_eq!(outcome.items[2].name, "Gone Mod");
    assert_eq!(outcome.items[2].wanted_version.as_deref(), Some("1"));
    assert_eq!(outcome.items[3].platform, Platform::CurseForge);
}

#[test]
fn a_snapshot_for_another_minecraft_version_or_loader_is_refused() {
    let other_version = Target {
        minecraft: "1.20.1".into(),
        loader: "fabric".into(),
    };
    assert_eq!(
        plan(&snapshot(), &other_version, &installed(), &HashMap::new()),
        Err(Refusal::Minecraft {
            snapshot: "1.21.1".into(),
            instance: "1.20.1".into(),
        })
    );
    let other_loader = Target {
        minecraft: "1.21.1".into(),
        loader: "forge".into(),
    };
    assert_eq!(
        plan(&snapshot(), &other_loader, &installed(), &HashMap::new()),
        Err(Refusal::Loader {
            snapshot: "fabric".into(),
            instance: "forge".into(),
        })
    );
}

#[test]
fn a_local_jar_is_missing_the_same_or_different_by_its_bytes() {
    let mut on_disk = HashMap::new();
    let outcome = plan(&snapshot(), &target(), &installed(), &on_disk).expect("the snapshot plans");
    assert_eq!(outcome.local[0].state, LocalState::Missing);

    on_disk.insert("custom.jar".to_string(), "AAAA".to_string());
    let outcome = plan(&snapshot(), &target(), &installed(), &on_disk).expect("the snapshot plans");
    assert_eq!(outcome.local[0].state, LocalState::Same, "hashes compare without regard to case");

    on_disk.insert("custom.jar".to_string(), "bbbb".to_string());
    let outcome = plan(&snapshot(), &target(), &installed(), &on_disk).expect("the snapshot plans");
    assert_eq!(outcome.local[0].state, LocalState::Differs);
}

#[test]
fn an_empty_snapshot_plans_nothing() {
    let outcome = plan(&Snapshot::default(), &target(), &installed(), &HashMap::new());
    assert_eq!(
        outcome,
        Err(Refusal::Minecraft {
            snapshot: String::new(),
            instance: "1.21.1".into(),
        })
    );
    let matching = Snapshot {
        minecraft: "1.21.1".into(),
        loader: "fabric".into(),
        ..Snapshot::default()
    };
    let outcome = plan(&matching, &target(), &installed(), &HashMap::new()).expect("the snapshot plans");
    assert!(outcome.items.is_empty() && outcome.local.is_empty());
    assert_eq!(outcome.changes(), 0);
}
