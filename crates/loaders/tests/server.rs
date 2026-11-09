use super::*;

/// A fresh, empty folder under the system temp directory, named for the test that asked for it.
fn fresh(name: &str) -> PathBuf {
    let dir = std::env::temp_dir().join(format!("lodestone-loaders-server-{name}-{}", std::process::id()));
    let _ = std::fs::remove_dir_all(&dir);
    std::fs::create_dir_all(&dir).expect("the folder can be made");
    dir
}

/// Writes an empty file at `relative` under `root`, making the folders it needs.
fn touch(root: &Path, relative: &str) {
    let path = root.join(relative);
    std::fs::create_dir_all(path.parent().expect("a file has a parent")).expect("the folders can be made");
    std::fs::write(path, b"").expect("the file can be written");
}

#[test]
fn a_jar_server_runs_java_on_the_jar_with_nogui() {
    let (shell, batch) = start_scripts(&Launch::Jar("server.jar".to_string()), None);
    assert!(shell.starts_with("#!/bin/sh\n"), "{shell}");
    assert!(shell.contains("JAVA=java\n"), "{shell}");
    assert!(shell.contains("exec \"$JAVA\" -jar server.jar nogui \"$@\""), "{shell}");
    assert!(batch.contains("set \"JAVA=java\""), "{batch}");
    assert!(batch.contains("\"%JAVA%\" -jar server.jar nogui %*"), "{batch}");
}

#[test]
fn an_argument_file_server_reads_the_jvm_arguments_and_its_platform_file() {
    let launch = Launch::Args {
        unix: "libraries/net/minecraftforge/forge/1.20.1-47.2.0/unix_args.txt".to_string(),
        windows: "libraries/net/minecraftforge/forge/1.20.1-47.2.0/win_args.txt".to_string(),
    };
    let (shell, batch) = start_scripts(&launch, None);
    assert!(
        shell.contains("exec \"$JAVA\" @user_jvm_args.txt @libraries/net/minecraftforge/forge/1.20.1-47.2.0/unix_args.txt nogui \"$@\""),
        "{shell}"
    );
    assert!(
        batch.contains("\"%JAVA%\" @user_jvm_args.txt @libraries/net/minecraftforge/forge/1.20.1-47.2.0/win_args.txt nogui %*"),
        "{batch}"
    );
}

#[test]
fn a_packed_linux_java_is_used_only_by_the_script_on_linux() {
    let java = PackedJava {
        program: "java/bin/java".to_string(),
        os: "Linux".to_string(),
    };
    let (shell, batch) = start_scripts(&Launch::Jar("server.jar".to_string()), Some(&java));
    assert!(
        shell.contains("if [ \"$(uname -s)\" = \"Linux\" ] && [ -x \"java/bin/java\" ]; then JAVA=\"java/bin/java\"; fi"),
        "{shell}"
    );
    assert!(!batch.contains("exist"), "the batch file keeps the system's Java: {batch}");
}

#[test]
fn a_packed_windows_java_is_checked_by_the_batch_file_only() {
    let java = PackedJava {
        program: "java/bin/java.exe".to_string(),
        os: "Windows".to_string(),
    };
    let (shell, batch) = start_scripts(&Launch::Jar("server.jar".to_string()), Some(&java));
    assert!(
        batch.contains("if exist \"java\\bin\\java.exe\" set \"JAVA=java\\bin\\java.exe\""),
        "{batch}"
    );
    assert!(!shell.contains("uname"), "the shell script keeps the system's Java: {shell}");
}

#[test]
fn a_packed_macos_java_is_checked_on_darwin() {
    let java = PackedJava {
        program: "java/jre.bundle/Contents/Home/bin/java".to_string(),
        os: "Darwin".to_string(),
    };
    let (shell, _batch) = start_scripts(&Launch::Jar("server.jar".to_string()), Some(&java));
    assert!(shell.contains("= \"Darwin\" ]"), "{shell}");
    assert!(shell.contains("-x \"java/jre.bundle/Contents/Home/bin/java\""), "{shell}");
}

#[test]
fn the_eula_is_true_only_when_the_player_accepted_it() {
    assert!(eula(true).lines().any(|line| line == "eula=true"));
    assert!(eula(false).lines().any(|line| line == "eula=false"));
    assert!(eula(false).contains("https://aka.ms/MinecraftEULA"));
}

#[test]
fn an_installed_forge_server_launches_from_the_argument_files_its_run_wrote() {
    let dir = fresh("forge-args");
    touch(&dir, "libraries/net/minecraftforge/forge/1.20.1-47.2.0/unix_args.txt");
    touch(&dir, "libraries/net/minecraftforge/forge/1.20.1-47.2.0/win_args.txt");
    assert_eq!(
        launch_of(&dir, "forge").expect("the server has a launch file"),
        Launch::Args {
            unix: "libraries/net/minecraftforge/forge/1.20.1-47.2.0/unix_args.txt".to_string(),
            windows: "libraries/net/minecraftforge/forge/1.20.1-47.2.0/win_args.txt".to_string(),
        }
    );
}

#[test]
fn an_older_forge_server_launches_from_the_jar_at_the_top_and_not_the_installer() {
    let dir = fresh("forge-jar");
    touch(&dir, "forge-1.12.2-14.23.5.2860-installer.jar");
    touch(&dir, "forge-1.12.2-14.23.5.2860.jar");
    assert_eq!(
        launch_of(&dir, "forge").expect("the server has a launch file"),
        Launch::Jar("forge-1.12.2-14.23.5.2860.jar".to_string())
    );
}

#[test]
fn the_installers_own_start_scripts_are_removed_and_nothing_else() {
    let dir = fresh("installer-scripts");
    touch(&dir, "run.sh");
    touch(&dir, "run.bat");
    touch(&dir, "user_jvm_args.txt");
    touch(&dir, "libraries/readme.txt");
    remove_installer_scripts(&dir);
    assert!(!dir.join("run.sh").exists());
    assert!(!dir.join("run.bat").exists());
    assert!(dir.join("user_jvm_args.txt").exists(), "the JVM arguments file is kept");
    assert!(dir.join("libraries/readme.txt").exists());
}

#[test]
fn a_server_with_no_launch_file_is_an_error() {
    let dir = fresh("no-launch");
    touch(&dir, "libraries/readme.txt");
    let error = launch_of(&dir, "neoforge").expect_err("nothing to launch");
    assert!(matches!(error, LoaderError::ServerLaunchMissing { loader: "neoforge", .. }), "{error:?}");
}
