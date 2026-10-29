use super::*;
use serde_json::json;

#[test]
fn minecraft_versions_map_to_their_neoforge_prefixes() {
    assert_eq!(minecraft_prefix("1.21.1"), "21.1.");
    assert_eq!(minecraft_prefix("1.20.4"), "20.4.");
    assert_eq!(minecraft_prefix("1.21"), "21.0.");
    assert_eq!(minecraft_prefix("25w14craftmine"), "0.25w14craftmine.");
    assert_eq!(minecraft_prefix("26.1"), "26.1.");
}

#[test]
fn pre_release_suffixes_are_unstable() {
    assert!(is_stable("21.1.95"));
    assert!(is_stable("26.3.0.94"));
    assert!(!is_stable("0.25w14craftmine.3-beta"));
    assert!(!is_stable("21.0.40-beta"));
    assert!(!is_stable("21.0.1-prerelease"));
}

#[test]
fn the_versions_envelope_parses() {
    // Recorded shape from the NeoForged maven API (first and last entries).
    let payload = json!({ "isSnapshot": false, "versions": ["0.25w14craftmine.3-beta", "20.4.237", "21.1.95", "26.3.0.94"] });
    let versions: NeoForgeVersions = serde_json::from_value(payload).expect("envelope parses");
    assert_eq!(versions.versions.len(), 4);
    assert_eq!(versions.versions[0], "0.25w14craftmine.3-beta");
    assert_eq!(versions.versions[3], "26.3.0.94");
}

#[test]
fn artifact_urls_cover_installer_and_universal() {
    let [(installer_url, installer_path), (universal_url, universal_path)] = artifact_urls("21.1.95");
    assert_eq!(
        installer_url,
        "https://maven.neoforged.net/releases/net/neoforged/neoforge/21.1.95/neoforge-21.1.95-installer.jar"
    );
    assert_eq!(installer_path, PathBuf::from("neoforge-21.1.95-installer.jar"));
    assert_eq!(
        universal_url,
        "https://maven.neoforged.net/releases/net/neoforged/neoforge/21.1.95/neoforge-21.1.95-universal.jar"
    );
    assert_eq!(universal_path, PathBuf::from("neoforge-21.1.95-universal.jar"));
}
