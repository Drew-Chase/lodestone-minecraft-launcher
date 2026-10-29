//! Builder + validation behavior (DESIGN.md §9).

mod common;

use std::path::PathBuf;

use common::{StubJava, builder_for};
use lodestone_launch::{LaunchError, Launcher};

#[test]
fn missing_java_executable_is_rejected() {
    let stub = StubJava::echo().unwrap();
    let missing = stub.dir.join("no-java").join("java.exe");
    let error = Launcher::builder(&missing, &stub.working_directory).build().unwrap_err();
    assert!(matches!(error, LaunchError::JavaNotFound { path } if path == missing));
}

#[test]
fn relative_java_path_is_rejected() {
    let stub = StubJava::echo().unwrap();
    let error = Launcher::builder("java", &stub.working_directory).build().unwrap_err();
    assert!(matches!(error, LaunchError::JavaNotFound { path } if path.as_os_str() == "java"));
}

#[test]
fn missing_working_directory_is_rejected() {
    let stub = StubJava::echo().unwrap();
    let missing = stub.dir.join("no-such-workdir");
    let error = Launcher::builder(&stub.java_path, &missing).build().unwrap_err();
    assert!(matches!(error, LaunchError::WorkingDirectoryMissing { path } if path == missing));
}

#[test]
fn empty_classpath_with_main_class_is_rejected() {
    let stub = StubJava::echo().unwrap();
    let error = builder_for(&stub).classpath(Vec::<PathBuf>::new()).build().unwrap_err();
    assert!(matches!(error, LaunchError::EmptyClasspath { .. }));
}

#[test]
fn unset_main_class_allows_empty_classpath() {
    // An empty `main_class` means "not set" (e.g. a future `-jar` mode): the
    // classpath rule is conditional on it, so this configuration is legal.
    let stub = StubJava::echo().unwrap();
    let launcher = Launcher::builder(&stub.java_path, &stub.working_directory).build();
    assert!(launcher.is_ok());
}

#[cfg(windows)]
#[test]
fn classpath_entry_with_invalid_character_is_rejected() {
    // Modern std on Windows rejects only `"` in entries: separator-bearing
    // entries are quoted into the join instead of rejected. Either way the
    // crate surfaces std's failure as `InvalidClasspath`.
    let stub = StubJava::echo().unwrap();
    let error = builder_for(&stub).classpath(["C:\\some\"dir"]).build().unwrap_err();
    assert!(matches!(error, LaunchError::InvalidClasspath { .. }));
}

#[cfg(not(windows))]
#[test]
fn classpath_entry_with_platform_separator_is_rejected() {
    let stub = StubJava::echo().unwrap();
    let error = builder_for(&stub).classpath(["/some:dir"]).build().unwrap_err();
    assert!(matches!(error, LaunchError::InvalidClasspath { .. }));
}

#[test]
fn launch_rechecks_java_after_build() {
    let stub = StubJava::echo().unwrap();
    let java_path = stub.java_path.clone();
    let launcher = builder_for(&stub).build().unwrap();
    std::fs::remove_file(&java_path).unwrap();
    let error = launcher.launch().unwrap_err();
    assert!(matches!(error, LaunchError::JavaNotFound { path } if path == java_path));
}

#[test]
fn getters_round_trip() {
    let stub = StubJava::echo().unwrap();
    let launcher = builder_for(&stub)
        .jvm_args(["-Xmx1G"])
        .game_args(["--game", "${classpath}"])
        .env([("STUB_VAR", "1")])
        .build()
        .unwrap();

    assert_eq!(launcher.java_path(), stub.java_path.as_path());
    assert_eq!(launcher.working_directory(), stub.working_directory.as_path());
    assert_eq!(launcher.library_directory(), stub.dir.join("libraries").as_path());
    assert_eq!(launcher.natives_directory(), Some(stub.dir.join("natives").as_path()));
    assert_eq!(launcher.assets_directory(), Some(stub.dir.join("assets").as_path()));
    assert_eq!(launcher.jvm_args(), ["-Xmx1G"].as_slice());
    assert_eq!(launcher.main_class(), "com.stub.Main");
    assert_eq!(launcher.classpath(), [stub.dir.join("one.jar"), stub.dir.join("two.jar")].as_slice());
    assert_eq!(launcher.game_args(), ["--game", "${classpath}"].as_slice());
    assert_eq!(launcher.env()["STUB_VAR"], "1");
    assert_eq!(launcher.launcher_name(), "lodestone-test");
    assert_eq!(launcher.launcher_version(), "0.0.0-test");
}
