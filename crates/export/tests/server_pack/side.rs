use super::*;
use crate::support::scratch;
use std::fs::File;
use std::io::Write;
use std::path::PathBuf;
use zip::write::SimpleFileOptions;

/// Writes a jar named `name` in `dir`, holding each `(entry, contents)` pair, and returns its path.
fn jar(dir: &Path, name: &str, entries: &[(&str, &str)]) -> PathBuf {
    let path = dir.join(name);
    let mut zip = zip::ZipWriter::new(File::create(&path).expect("the jar can be created"));
    for (entry, contents) in entries {
        zip.start_file(*entry, SimpleFileOptions::default()).expect("the entry starts");
        zip.write_all(contents.as_bytes()).expect("the entry writes");
    }
    zip.finish().expect("the jar closes");
    path
}

#[test]
fn a_fabric_client_environment_is_client_only() {
    let dir = scratch("side-fabric-client");
    let path = jar(&dir, "client.jar", &[("fabric.mod.json", r#"{"id":"clientmod","environment":"client"}"#)]);
    let detection = detect(&path);
    assert_eq!(detection.side, Some(Side::ClientOnly));
    assert_eq!(detection.loader.as_deref(), Some("fabric"));
    assert_eq!(detection.mod_id.as_deref(), Some("clientmod"));
}

#[test]
fn a_fabric_wildcard_environment_is_both_sides() {
    let dir = scratch("side-fabric-wildcard");
    let path = jar(&dir, "both.jar", &[("fabric.mod.json", r#"{"id":"bothmod","environment":"*"}"#)]);
    assert_eq!(detect(&path).side, Some(Side::ClientAndServer));
}

#[test]
fn a_forge_both_side_comes_from_the_loader_dependency() {
    let dir = scratch("side-forge-both");
    let toml = "modLoader=\"javafml\"\n[[mods]]\nmodId=\"xaero\"\n[[dependencies.xaero]]\nmodId=\"forge\"\nmandatory=true\nside=\"BOTH\"\n";
    let path = jar(&dir, "forge.jar", &[("META-INF/mods.toml", toml)]);
    let detection = detect(&path);
    assert_eq!(detection.side, Some(Side::ClientAndServer));
    assert_eq!(detection.loader.as_deref(), Some("forge"));
    assert_eq!(detection.mod_id.as_deref(), Some("xaero"));
}

#[test]
fn a_neoforge_client_side_is_client_only() {
    let dir = scratch("side-neoforge-client");
    let toml =
        "modLoader=\"javafml\"\n[[mods]]\nmodId=\"appleskin\"\n[[dependencies.appleskin]]\nmodId=\"neoforge\"\ntype=\"required\"\nside=\"CLIENT\"\n";
    let path = jar(&dir, "neoforge.jar", &[("META-INF/neoforge.mods.toml", toml)]);
    let detection = detect(&path);
    assert_eq!(detection.side, Some(Side::ClientOnly));
    assert_eq!(detection.loader.as_deref(), Some("neoforge"));
}

#[test]
fn a_quilt_mod_without_an_environment_runs_on_both_sides() {
    let dir = scratch("side-quilt-default");
    let path = jar(
        &dir,
        "quilt.jar",
        &[("quilt.mod.json", r#"{"schema_version":1,"quilt_loader":{"id":"qmod"}}"#)],
    );
    let detection = detect(&path);
    assert_eq!(detection.side, Some(Side::ClientAndServer));
    assert_eq!(detection.loader.as_deref(), Some("quilt"));
    assert_eq!(detection.mod_id.as_deref(), Some("qmod"));
    assert_eq!(detection.evidence, "quilt.mod.json → both");
}

#[test]
fn a_quilt_dedicated_server_environment_is_server_only() {
    let dir = scratch("side-quilt-server");
    let json = r#"{"quilt_loader":{"id":"qserver"},"minecraft":{"environment":"dedicated_server"}}"#;
    let path = jar(&dir, "server.jar", &[("quilt.mod.json", json)]);
    assert_eq!(detect(&path).side, Some(Side::ServerOnly));
}

#[test]
fn a_quilt_client_environment_is_client_only() {
    let dir = scratch("side-quilt-client");
    let json = r#"{"quilt_loader":{"id":"qclient"},"minecraft":{"environment":"client"}}"#;
    let path = jar(&dir, "client.jar", &[("quilt.mod.json", json)]);
    assert_eq!(detect(&path).side, Some(Side::ClientOnly));
}

#[test]
fn an_unknown_quilt_environment_stays_unsure_and_says_why() {
    let dir = scratch("side-quilt-unknown");
    let json = r#"{"quilt_loader":{"id":"qodd"},"minecraft":{"environment":"everywhere"}}"#;
    let path = jar(&dir, "odd.jar", &[("quilt.mod.json", json)]);
    let detection = detect(&path);
    assert_eq!(detection.side, None);
    assert_eq!(detection.loader.as_deref(), Some("quilt"));
    assert!(detection.evidence.contains("unknown environment"), "{}", detection.evidence);
}

#[test]
fn a_fabric_manifest_that_names_a_side_wins_over_quilt() {
    let dir = scratch("side-fabric-first");
    let fabric = r#"{"id":"dual","environment":"server"}"#;
    let quilt = r#"{"quilt_loader":{"id":"dual"},"minecraft":{"environment":"client"}}"#;
    let path = jar(&dir, "dual.jar", &[("fabric.mod.json", fabric), ("quilt.mod.json", quilt)]);
    let detection = detect(&path);
    assert_eq!(detection.side, Some(Side::ServerOnly));
    assert_eq!(detection.loader.as_deref(), Some("fabric"));
}

#[test]
fn a_jar_with_no_side_metadata_is_unsure() {
    let dir = scratch("side-none");
    let path = jar(&dir, "plain.jar", &[("readme.txt", "hello")]);
    let detection = detect(&path);
    assert_eq!(detection.side, None);
    assert_eq!(detection.loader, None);
    assert_eq!(detection.evidence, "no side metadata found");
}

#[test]
fn a_file_that_is_not_a_jar_is_unsure_rather_than_an_error() {
    let dir = scratch("side-not-a-jar");
    let path = dir.join("broken.jar");
    std::fs::write(&path, b"not a zip archive").expect("the file is written");
    let detection = detect(&path);
    assert_eq!(detection.side, None);
    assert_eq!(detection.loader, None);
}
