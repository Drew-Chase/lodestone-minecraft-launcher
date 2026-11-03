use super::*;

fn temp_path(tag: &str) -> PathBuf {
    std::env::temp_dir().join(format!("lodestone-storage-test-{tag}-{}.json", std::process::id()))
}

fn microsoft(username: &str, token: &str) -> StoredAccount {
    StoredAccount::Microsoft(StoredMicrosoft {
        refresh_token: token.into(),
        username: username.into(),
    })
}

fn offline(username: &str) -> StoredAccount {
    StoredAccount::Offline { username: username.into() }
}

#[test]
fn credentials_round_trip() {
    let path = temp_path("round-trip");
    let credentials = StoredCredentials {
        accounts: vec![microsoft("Steve", "refresh-token-value"), offline("Drew")],
        primary: 1,
        ..StoredCredentials::default()
    };
    StoredCredentials::store_to(&path, &credentials).expect("store succeeds");
    let loaded = StoredCredentials::load_from(&path);
    assert_eq!(loaded, credentials);
    std::fs::remove_file(&path).ok();
}

#[test]
fn a_missing_file_loads_as_empty() {
    let path = temp_path("missing");
    let loaded = StoredCredentials::load_from(&path);
    assert_eq!(loaded, StoredCredentials::default());
}

#[test]
fn a_corrupt_file_loads_as_empty() {
    let path = temp_path("corrupt");
    std::fs::write(&path, "{ not json").expect("write succeeds");
    let loaded = StoredCredentials::load_from(&path);
    assert_eq!(loaded, StoredCredentials::default());
    std::fs::remove_file(&path).ok();
}

#[test]
fn an_empty_store_serializes_without_secrets() {
    let json = serde_json::to_string(&StoredCredentials::default()).expect("serializes");
    assert_eq!(json, "{\"accounts\":[],\"primary\":0}");
}

#[test]
fn a_file_from_one_account_era_moves_both_sign_ins_into_the_list() {
    let path = temp_path("legacy");
    std::fs::write(
        &path,
        r#"{"microsoft":{"refresh_token":"token-1","username":"Steve"},"offline_username":"Drew"}"#,
    )
    .expect("write succeeds");
    let loaded = StoredCredentials::load_from(&path);
    std::fs::remove_file(&path).ok();
    assert_eq!(loaded.accounts, vec![microsoft("Steve", "token-1"), offline("Drew")]);
    assert_eq!(loaded.primary, 0);
}

#[test]
fn a_legacy_name_already_in_the_list_is_not_added_twice() {
    let mut credentials = StoredCredentials {
        accounts: vec![microsoft("Steve", "newer-token")],
        microsoft: Some(StoredMicrosoft {
            refresh_token: "older-token".into(),
            username: "Steve".into(),
        }),
        ..StoredCredentials::default()
    };
    credentials.migrate();
    assert_eq!(credentials.accounts, vec![microsoft("Steve", "newer-token")]);
}

#[test]
fn the_primary_is_the_first_account_when_its_index_is_past_the_end() {
    let credentials = StoredCredentials {
        accounts: vec![offline("A"), offline("B")],
        primary: 7,
        ..StoredCredentials::default()
    };
    assert_eq!(credentials.primary_index(), Some(0));
    assert_eq!(credentials.primary_account(), Some(&offline("A")));
    assert_eq!(StoredCredentials::default().primary_index(), None);
}

#[test]
fn adding_an_account_makes_it_the_primary_and_replaces_one_of_the_same_name() {
    let mut credentials = StoredCredentials::default();
    assert_eq!(credentials.add_and_make_primary(microsoft("Steve", "token-1")), 0);
    assert_eq!(credentials.add_and_make_primary(offline("Drew")), 1);
    assert_eq!(credentials.add_and_make_primary(microsoft("Steve", "token-2")), 0);
    assert_eq!(credentials.accounts, vec![microsoft("Steve", "token-2"), offline("Drew")]);
    assert_eq!(credentials.primary, 0);
}

#[test]
fn removing_an_account_keeps_the_default_on_an_account_that_remains() {
    let mut credentials = StoredCredentials {
        accounts: vec![offline("A"), offline("B"), offline("C")],
        primary: 1,
        ..StoredCredentials::default()
    };
    assert_eq!(credentials.remove(0), Some(offline("A")));
    assert_eq!(credentials.primary_account(), Some(&offline("B")));
    assert_eq!(credentials.remove(1), Some(offline("C")));
    assert_eq!(credentials.primary_account(), Some(&offline("B")));
    assert_eq!(credentials.remove(0), Some(offline("B")));
    assert_eq!(credentials.primary_index(), None);
    assert_eq!(credentials.remove(0), None);
}

#[test]
fn a_refresh_token_is_replaced_for_the_named_microsoft_account_only() {
    let mut credentials = StoredCredentials {
        accounts: vec![offline("Steve"), microsoft("Steve", "old")],
        ..StoredCredentials::default()
    };
    assert!(credentials.replace_refresh_token("Steve", "new".into()));
    assert_eq!(credentials.accounts, vec![offline("Steve"), microsoft("Steve", "new")]);
    assert!(!credentials.replace_refresh_token("Alex", "x".into()));
}

#[test]
fn the_last_offline_name_is_the_latest_offline_account() {
    let credentials = StoredCredentials {
        accounts: vec![offline("First"), microsoft("Steve", "t"), offline("Second")],
        ..StoredCredentials::default()
    };
    assert_eq!(credentials.last_offline_name(), Some("Second"));
    assert_eq!(StoredCredentials::default().last_offline_name(), None);
}

#[test]
fn a_save_during_a_write_waits_for_it() {
    let mut writer = CredentialWriter::default();
    assert!(writer.request(), "an idle writer starts the write at once");
    assert!(!writer.request(), "a save during a write only queues");
    assert!(!writer.request(), "further saves coalesce into the queued one");
    assert!(writer.complete(), "the queued save starts as the write finishes");
    assert!(!writer.complete(), "with nothing queued the writer goes idle");
}

#[test]
fn an_idle_writer_starts_the_next_save_at_once() {
    let mut writer = CredentialWriter::default();
    assert!(writer.request());
    assert!(!writer.complete());
    assert!(writer.request(), "after going idle, a save starts a new write");
}
