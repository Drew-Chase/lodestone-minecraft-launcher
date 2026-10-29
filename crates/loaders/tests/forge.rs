use super::*;

/// Recorded slice of the real `maven-metadata.json`: the map is keyed by
/// Minecraft version, and the values are the full combined version strings
/// (including odd suffixes like `1.7.10-10.13.4.1614-1.7.10`).
const SAMPLE_INDEX: &str = r#"{
    "1.1": ["1.1-1.3.2.1", "1.1-1.3.4.29"],
    "1.7.10": ["1.7.10-10.13.0.1150", "1.7.10-10.13.4.1614-1.7.10"],
    "1.21.1": ["1.21.1-52.0.0", "1.21.1-52.1.20"]
}"#;

#[test]
fn the_index_parses_into_a_sorted_map() {
    let index: BTreeMap<String, Vec<String>> = serde_json::from_str(SAMPLE_INDEX).expect("real metadata shape");
    let keys: Vec<&str> = index.keys().map(String::as_str).collect();
    assert_eq!(keys, ["1.1", "1.21.1", "1.7.10"]);
    assert_eq!(index["1.21.1"].len(), 2);
}

#[test]
fn combined_versions_embed_their_minecraft_version() {
    assert_eq!(minecraft_from_combined("1.21.1-52.1.20").as_deref(), Some("1.21.1"));
    assert_eq!(minecraft_from_combined("1.7.10-10.13.4.1614-1.7.10").as_deref(), Some("1.7.10"));
    assert_eq!(minecraft_from_combined("nonsense"), None);
}

#[test]
fn artifact_urls_cover_installer_and_universal() {
    let [(installer_url, installer_path), (universal_url, universal_path)] = artifact_urls("1.7.10-10.13.4.1614-1.7.10");
    assert_eq!(
        installer_url,
        "https://maven.minecraftforge.net/net/minecraftforge/forge/1.7.10-10.13.4.1614-1.7.10/forge-1.7.10-10.13.4.1614-1.7.10-installer.jar"
    );
    assert_eq!(installer_path, PathBuf::from("forge-1.7.10-10.13.4.1614-1.7.10-installer.jar"));
    assert_eq!(
        universal_url,
        "https://maven.minecraftforge.net/net/minecraftforge/forge/1.7.10-10.13.4.1614-1.7.10/forge-1.7.10-10.13.4.1614-1.7.10-universal.jar"
    );
    assert_eq!(universal_path, PathBuf::from("forge-1.7.10-10.13.4.1614-1.7.10-universal.jar"));
}

#[test]
fn versions_for_an_unknown_minecraft_version_are_empty() {
    let index: BTreeMap<String, Vec<String>> = serde_json::from_str(SAMPLE_INDEX).expect("real metadata shape");
    assert!(!index.contains_key("2.0.0"));
}
