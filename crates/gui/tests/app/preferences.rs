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
    assert_eq!(json, "{\"language\":null,\"discover_source\":null}");
}

#[test]
fn a_file_without_the_discover_platform_still_loads() {
    let preferences: Preferences = serde_json::from_str("{\"language\":\"en_us\"}").expect("older files parse");
    assert_eq!(preferences.language.as_deref(), Some("en_us"));
    assert_eq!(preferences.discover_source, None);
}
