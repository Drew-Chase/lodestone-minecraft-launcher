use super::*;

/// A fresh folder for one test, named after the test so parallel runs do not share it.
fn scratch(name: &str) -> PathBuf {
    let dir = std::env::temp_dir().join(format!("lodestone-loaders-{}-{name}", std::process::id()));
    let _ = std::fs::remove_dir_all(&dir);
    std::fs::create_dir_all(&dir).expect("the scratch folder creates");
    dir
}

#[test]
fn write_version_places_the_json_under_versions() {
    let game_dir = scratch("write-version");
    let json = serde_json::json!({ "id": "fabric-loader-0.16.10-1.21.4", "mainClass": "net.fabricmc.loader.impl.launch.knot.KnotClient" });

    let installed = write_version(&game_dir, "fabric-loader-0.16.10-1.21.4", &json).expect("the version writes");

    assert_eq!(installed.id, "fabric-loader-0.16.10-1.21.4");
    assert_eq!(
        installed.json,
        game_dir.join("versions/fabric-loader-0.16.10-1.21.4/fabric-loader-0.16.10-1.21.4.json")
    );
    let written: serde_json::Value =
        serde_json::from_str(&std::fs::read_to_string(&installed.json).expect("the json reads")).expect("the json parses");
    assert_eq!(written, json);
    let _ = std::fs::remove_dir_all(&game_dir);
}

#[test]
fn seeding_creates_a_launcher_profile_only_when_missing() {
    let game_dir = scratch("seed-profiles");
    seed_launcher_profiles(&game_dir).expect("the profile seeds");
    let profile = game_dir.join("launcher_profiles.json");
    assert_eq!(std::fs::read_to_string(&profile).expect("the profile reads"), r#"{"profiles":{}}"#);

    std::fs::write(&profile, "kept").expect("the test writes its own profile");
    seed_launcher_profiles(&game_dir).expect("seeding again succeeds");
    assert_eq!(
        std::fs::read_to_string(&profile).expect("the profile reads"),
        "kept",
        "an existing profile is left alone"
    );
    let _ = std::fs::remove_dir_all(&game_dir);
}

#[test]
fn the_installer_is_found_among_the_downloaded_jars() {
    let jars = [
        PathBuf::from("libraries/forge-1.20.1-47.3.0-universal.jar"),
        PathBuf::from("libraries/forge-1.20.1-47.3.0-installer.jar"),
    ];
    assert_eq!(installer_jar(&jars), Some(&jars[1]));
    assert_eq!(installer_jar(&jars[..1]), None);
}

#[cfg(unix)]
fn write_script(path: &Path, body: &str) {
    use std::os::unix::fs::PermissionsExt;
    std::fs::write(path, format!("#!/bin/sh\n{body}\n")).expect("the script writes");
    std::fs::set_permissions(path, std::fs::Permissions::from_mode(0o755)).expect("the script becomes executable");
}

#[cfg(unix)]
#[test]
fn an_installer_run_reports_the_version_it_created_and_streams_its_output() {
    let game_dir = scratch("installer-success");
    let java = game_dir.join("fake-java");
    write_script(
        &java,
        r#"echo "installing $3"
mkdir -p "$4/versions/fake-1.0"
echo '{"id":"fake-1.0","inheritsFrom":"1.0"}' > "$4/versions/fake-1.0/fake-1.0.json"
echo "done" >&2"#,
    );
    let installer = game_dir.join("installer.jar");
    let (sender, mut receiver) = tokio::sync::mpsc::channel::<String>(16);

    let installed =
        run_installer("forge", "1.0", java, installer, "--installClient", game_dir.clone(), Some(sender)).expect("the installer succeeds");

    assert_eq!(installed.id, "fake-1.0");
    assert!(installed.json.is_file());
    let mut lines = Vec::new();
    while let Ok(line) = receiver.try_recv() {
        lines.push(line);
    }
    assert_eq!(lines, ["installing --installClient", "done"]);
    let _ = std::fs::remove_dir_all(&game_dir);
}

#[cfg(unix)]
#[test]
fn a_failed_installer_reports_its_exit_code_and_last_lines() {
    let game_dir = scratch("installer-failure");
    let java = game_dir.join("fake-java");
    write_script(
        &java,
        r#"echo "no network" >&2
exit 3"#,
    );

    let error = run_installer(
        "neoforge",
        "1.0",
        java,
        game_dir.join("installer.jar"),
        "--install-client",
        game_dir.clone(),
        None,
    )
    .expect_err("a non-zero exit fails the run");

    match error {
        LoaderError::InstallerFailed { loader, status, output } => {
            assert_eq!(loader, "neoforge");
            assert_eq!(status, "exit code 3");
            assert!(output.contains("no network"), "the failure carries the installer's output: {output}");
        }
        other => panic!("expected InstallerFailed, got {other:?}"),
    }
    let _ = std::fs::remove_dir_all(&game_dir);
}

#[cfg(unix)]
#[test]
fn a_vanilla_folder_alone_is_not_the_loader_version() {
    let game_dir = scratch("installer-vanilla-only");
    let java = game_dir.join("fake-java");
    write_script(
        &java,
        r#"mkdir -p "$4/versions/1.0"
echo '{"id":"1.0"}' > "$4/versions/1.0/1.0.json""#,
    );

    let error = run_installer(
        "forge",
        "1.0",
        java,
        game_dir.join("installer.jar"),
        "--installClient",
        game_dir.clone(),
        None,
    )
    .expect_err("the vanilla folder does not count as the loader");

    assert!(
        matches!(error, LoaderError::InstallerVersionMissing { loader: "forge", .. }),
        "got {error:?}"
    );
    let _ = std::fs::remove_dir_all(&game_dir);
}

#[cfg(unix)]
#[test]
fn an_installer_that_creates_no_version_is_reported() {
    let game_dir = scratch("installer-no-version");
    let java = game_dir.join("fake-java");
    write_script(&java, "exit 0");

    let error = run_installer(
        "forge",
        "1.0",
        java,
        game_dir.join("installer.jar"),
        "--installClient",
        game_dir.clone(),
        None,
    )
    .expect_err("a run without a new version fails");

    assert!(
        matches!(error, LoaderError::InstallerVersionMissing { loader: "forge", .. }),
        "got {error:?}"
    );
    let _ = std::fs::remove_dir_all(&game_dir);
}

#[test]
fn a_missing_java_program_is_reported_as_a_launch_failure() {
    let game_dir = scratch("installer-no-java");
    let error = run_installer(
        "forge",
        "1.0",
        game_dir.join("no-such-java"),
        game_dir.join("installer.jar"),
        "--installClient",
        game_dir.clone(),
        None,
    )
    .expect_err("a missing program cannot run");

    assert!(matches!(error, LoaderError::InstallerLaunch { .. }), "got {error:?}");
    let _ = std::fs::remove_dir_all(&game_dir);
}
