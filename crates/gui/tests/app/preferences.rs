use super::*;

fn temp_path(tag: &str) -> PathBuf {
    std::env::temp_dir().join(format!("lodestone-preferences-test-{tag}-{}.json", std::process::id()))
}

#[test]
fn preferences_round_trip() {
    let path = temp_path("round-trip");
    let preferences = Preferences {
        language: Some(String::from("ru_ru")),
        discover_source: Some(String::from("ftb")),
        ..Preferences::default()
    };
    Preferences::store_to(&path, &preferences).expect("store succeeds");
    assert_eq!(Preferences::load_from(&path), preferences);
    std::fs::remove_file(&path).ok();
}

#[test]
fn a_missing_file_loads_the_defaults() {
    assert_eq!(Preferences::load_from(&temp_path("missing")), Preferences::default());
}

#[test]
fn a_corrupt_file_loads_the_defaults() {
    let path = temp_path("corrupt");
    std::fs::write(&path, "{ not json").expect("write succeeds");
    assert_eq!(Preferences::load_from(&path), Preferences::default());
    std::fs::remove_file(&path).ok();
}

#[test]
fn an_unset_language_serializes_as_null() {
    let json = serde_json::to_string(&Preferences::default()).expect("serializes");
    assert!(
        json.starts_with("{\"language\":null,\"discover_source\":null,\"instance_defaults\":{"),
        "{json}"
    );
}

#[test]
fn a_file_without_the_discover_platform_still_loads() {
    let preferences: Preferences = serde_json::from_str("{\"language\":\"en_us\"}").expect("older files parse");
    assert_eq!(preferences.language.as_deref(), Some("en_us"));
    assert_eq!(preferences.discover_source, None);
}

#[test]
fn the_settings_values_sit_beside_the_language_in_the_file() {
    let json = serde_json::to_string(&Preferences::default()).expect("serializes");
    assert!(json.contains("\"startup_page\":\"library\""), "{json}");
    assert!(json.contains("\"auto_update\":true"), "{json}");
}

#[test]
fn a_file_from_before_the_settings_page_keeps_its_launch_defaults() {
    let preferences: Preferences = serde_json::from_str(r#"{"language":"en_us","instance_defaults":{"memory_mb":8192}}"#).expect("older files parse");
    assert_eq!(preferences.language.as_deref(), Some("en_us"));
    assert_eq!(preferences.settings.instance_defaults.memory_mb, 8192);
    assert!(preferences.settings.auto_update);
    assert_eq!(preferences.settings.concurrent_downloads, 64);
    assert_eq!(preferences.settings.network.timeout_secs, 30);
}

#[test]
fn removing_the_file_deletes_it_and_a_missing_file_is_not_an_error() {
    let path = temp_path("remove");
    Preferences::store_to(&path, &Preferences::default()).expect("store succeeds");
    Preferences::remove_from(&path).expect("removal succeeds");
    assert!(!path.exists());
    Preferences::remove_from(&path).expect("a missing file is not an error");
}

#[test]
fn the_news_startup_page_is_saved_by_its_name() {
    let preferences = Preferences {
        settings: SettingsValues {
            startup_page: StartupPage::News,
            ..SettingsValues::default()
        },
        ..Preferences::default()
    };
    let json = serde_json::to_string(&preferences).expect("serializes");
    assert!(json.contains("\"startup_page\":\"news\""), "{json}");
    let loaded: Preferences = serde_json::from_str(&json).expect("parses");
    assert_eq!(loaded.settings.startup_page, StartupPage::News);
}

#[test]
fn located_launcher_folders_round_trip_and_an_older_file_loads_without_them() {
    let path = temp_path("launcher-folders");
    let mut preferences = Preferences::default();
    preferences
        .launcher_folders
        .insert("atlauncher".to_string(), PathBuf::from("D:/Games/ATLauncher"));
    Preferences::store_to(&path, &preferences).expect("store succeeds");
    assert_eq!(Preferences::load_from(&path).launcher_folders, preferences.launcher_folders);
    std::fs::write(&path, r#"{"language":"en_us"}"#).expect("write succeeds");
    assert!(Preferences::load_from(&path).launcher_folders.is_empty());
    std::fs::remove_file(&path).ok();
}
