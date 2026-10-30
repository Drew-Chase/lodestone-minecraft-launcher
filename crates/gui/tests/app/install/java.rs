use super::*;

/// A fresh folder for one test, named after the test so parallel runs do not share it.
fn scratch(name: &str) -> PathBuf {
    let dir = std::env::temp_dir().join(format!("lodestone-java-{}-{name}", std::process::id()));
    let _ = std::fs::remove_dir_all(&dir);
    std::fs::create_dir_all(&dir).expect("the scratch folder creates");
    dir
}

#[test]
fn components_take_their_short_folder_names() {
    assert_eq!(short_name("java-runtime-delta"), "delta");
    assert_eq!(short_name("java-runtime-gamma-snapshot"), "gamma-snapshot");
    assert_eq!(short_name("jre-legacy"), "legacy");
    assert_eq!(short_name("something-else"), "something-else");
}

#[test]
fn the_java_program_sits_where_each_platform_puts_it() {
    let home = Path::new("/meta/java/delta");
    let expected = if cfg!(target_os = "windows") {
        home.join("bin").join("java.exe")
    } else if cfg!(target_os = "macos") {
        home.join("jre.bundle").join("Contents").join("Home").join("bin").join("java")
    } else {
        home.join("bin").join("java")
    };
    assert_eq!(binary(home), expected);
}

#[test]
fn the_file_list_keeps_its_flags_and_links() {
    let text = r#"{"files":{
        "bin/java":{"type":"file","executable":true,"downloads":{}},
        "lib/libjli.so":{"type":"link","target":"../jli/libjli.so"},
        "lib":{"type":"directory"},
        "release":{"type":"file"}}}"#;
    let entries = parse_entries(text).expect("the list parses");
    let find = |name: &str| entries.iter().find(|entry| entry.name == name).expect("the entry is listed");

    assert!(find("bin/java").executable);
    assert_eq!(find("lib/libjli.so").kind, "link");
    assert_eq!(find("lib/libjli.so").target.as_deref(), Some("../jli/libjli.so"));
    assert!(!find("release").executable);
    assert_eq!(find("lib").kind, "directory");
}

#[test]
fn a_file_list_without_files_is_rejected() {
    assert!(parse_entries(r#"{"other":1}"#).is_err());
}

#[cfg(unix)]
#[test]
fn applying_the_list_makes_executables_directories_and_links() {
    use std::os::unix::fs::PermissionsExt;

    let root = scratch("apply");
    std::fs::create_dir_all(root.join("bin")).expect("the bin folder creates");
    std::fs::write(root.join("bin/java"), b"#!/bin/sh").expect("the program writes");
    let entries = vec![
        Entry {
            name: "bin/java".to_string(),
            kind: "file".to_string(),
            executable: true,
            target: None,
        },
        Entry {
            name: "lib/libjli.so".to_string(),
            kind: "link".to_string(),
            executable: false,
            target: Some("../jli/libjli.so".to_string()),
        },
        Entry {
            name: "lib".to_string(),
            kind: "directory".to_string(),
            executable: false,
            target: None,
        },
    ];

    apply(&root, &entries).expect("the flags apply");

    let mode = std::fs::metadata(root.join("bin/java")).expect("the program exists").permissions().mode();
    assert_eq!(mode & 0o777, 0o755);
    assert!(root.join("lib").is_dir());
    assert_eq!(
        std::fs::read_link(root.join("lib/libjli.so")).expect("the link exists"),
        PathBuf::from("../jli/libjli.so")
    );
    let _ = std::fs::remove_dir_all(&root);
}

#[test]
fn major_versions_come_from_the_leading_number() {
    assert_eq!(major("21.0.5"), Some(21));
    assert_eq!(major("17.0.8.1+1"), Some(17));
    assert_eq!(major("8u51"), None, "legacy names carry no major number");
}

/// Downloads a real runtime and checks that it runs once the flags are applied. It fetches
/// about 95 MB from Mojang, so it only runs when asked: `cargo test -- --ignored`.
#[tokio::test]
#[ignore = "downloads a Java runtime from Mojang (about 95 MB)"]
async fn an_installed_runtime_runs_once_its_flags_are_applied() {
    let manifest = JavaManifest::fetch().await.expect("the runtime list fetches");
    let runtimes = platform_runtimes(&manifest).expect("this platform publishes runtimes");
    let runtime = runtime(runtimes, "java-runtime-gamma").expect("gamma is published here");
    let home = scratch("e2e");

    runtime.install(&home, PARALLEL, None).await.expect("the runtime downloads");
    apply_manifest_flags(runtime, &home).await.expect("the flags apply");

    let output = std::process::Command::new(binary(&home))
        .arg("-version")
        .output()
        .expect("the java program starts");
    assert!(
        output.status.success(),
        "java -version failed: {}",
        String::from_utf8_lossy(&output.stderr)
    );
    let _ = std::fs::remove_dir_all(&home);
}
