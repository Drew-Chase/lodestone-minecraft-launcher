use super::*;

fn temp_path(tag: &str) -> PathBuf {
    std::env::temp_dir().join(format!("lodestone-storage-test-{tag}-{}.json", std::process::id()))
}

#[test]
fn credentials_round_trip() {
    let path = temp_path("round-trip");
    let credentials = StoredCredentials {
        microsoft: Some(StoredMicrosoft {
            refresh_token: "refresh-token-value".into(),
            username: "Steve".into(),
        }),
        offline_username: Some("Drew".into()),
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
    assert_eq!(json, "{\"microsoft\":null,\"offline_username\":null}");
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
