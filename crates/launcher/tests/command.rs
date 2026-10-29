use std::env;
use std::path::PathBuf;

use super::*;

/// A valid launcher around real files (this test binary and its directory),
/// so `build()`'s filesystem validation passes. Nothing here is ever
/// executed: `build_command` is pure. Returns the config and the directory
/// its derived paths hang off.
fn launcher(game_args: &[&str], natives: bool) -> (Launcher, PathBuf) {
    let exe = env::current_exe().expect("the test binary has a path");
    let dir = exe.parent().expect("the test binary has a parent").to_path_buf();
    let mut builder = Launcher::builder(&exe, &dir)
        .library_directory(dir.join("libraries"))
        .assets_directory(dir.join("assets"))
        .main_class("com.example.Main")
        .classpath([dir.join("one.jar"), dir.join("two.jar")])
        .jvm_args(["-Xmx1G"])
        .launcher_name("test-launcher")
        .launcher_version("1.2.3");
    if natives {
        builder = builder.natives_directory(dir.join("natives"));
    }
    let config = builder.game_args(game_args.to_vec()).build().expect("the description is valid");
    (config, dir)
}

/// The command's args after the program, as lossy strings.
fn args_of(command: &Command) -> Vec<String> {
    command.get_args().map(|arg| arg.to_string_lossy().into_owned()).collect()
}

#[test]
fn argument_order_follows_the_contract() {
    let (config, dir) = launcher(&["--game-flag"], true);
    let command = build_command(&config).expect("classpath entries are joinable");

    let natives = format!("-Djava.library.path={}", dir.join("natives").display());
    let classpath = format!("{}{CLASSPATH_SEPARATOR}{}", dir.join("one.jar").display(), dir.join("two.jar").display());
    let expected = vec![
        "-Xmx1G".to_string(),
        natives,
        "-cp".to_string(),
        classpath,
        "com.example.Main".to_string(),
        "--game-flag".to_string(),
    ];

    assert_eq!(command.get_program().to_string_lossy(), env::current_exe().unwrap().to_string_lossy());
    assert_eq!(command.get_current_dir(), Some(dir.as_path()));
    assert_eq!(args_of(&command), expected);
}

#[test]
fn classpath_joins_with_the_platform_separator() {
    let (config, dir) = launcher(&[], false);
    let command = build_command(&config).unwrap();
    let args = args_of(&command);

    let cp_index = args.iter().position(|arg| arg == "-cp").expect("-cp is emitted");
    let joined = &args[cp_index + 1];
    assert_eq!(
        joined.matches(CLASSPATH_SEPARATOR).count(),
        1,
        "two entries joined by one separator, got {joined:?}"
    );
    assert!(joined.starts_with(&dir.join("one.jar").display().to_string()));
    assert!(joined.ends_with(&dir.join("two.jar").display().to_string()));
}

#[test]
fn environmental_placeholders_expand_to_configured_values() {
    let (config, dir) = launcher(
        &[
            "${classpath}",
            "${classpath_separator}",
            "${library_directory}",
            "${natives_directory}",
            "${assets_root}",
            "${launcher_name}",
            "${launcher_version}",
        ],
        true,
    );
    let command = build_command(&config).unwrap();
    let args = args_of(&command);

    let classpath = format!("{}{CLASSPATH_SEPARATOR}{}", dir.join("one.jar").display(), dir.join("two.jar").display());
    let main_index = args.iter().position(|arg| arg == "com.example.Main").expect("main class is emitted");
    assert_eq!(
        &args[main_index + 1..],
        &[
            classpath.as_str(),
            CLASSPATH_SEPARATOR.to_string().as_str(),
            dir.join("libraries").to_str().unwrap(),
            dir.join("natives").to_str().unwrap(),
            dir.join("assets").to_str().unwrap(),
            "test-launcher",
            "1.2.3",
        ][..]
    );
}

#[test]
fn unknown_unset_and_substring_placeholders_pass_through_untouched() {
    // No natives directory configured: `${natives_directory}` is known but
    // unset, so it passes through like an unknown token. A non-whole-token
    // occurrence is never substituted.
    let (config, _dir) = launcher(&["${not_a_placeholder}", "${natives_directory}", "prefix-${launcher_name}"], false);
    let command = build_command(&config).unwrap();
    let args = args_of(&command);

    let main_index = args.iter().position(|arg| arg == "com.example.Main").expect("main class is emitted");
    assert_eq!(
        &args[main_index + 1..],
        &["${not_a_placeholder}", "${natives_directory}", "prefix-${launcher_name}"][..]
    );
}

#[test]
fn environment_is_passed_through_deterministically() {
    let exe = env::current_exe().unwrap();
    let dir = exe.parent().unwrap();
    let config = Launcher::builder(&exe, dir)
        .main_class("com.example.Main")
        .classpath([dir.join("one.jar")])
        .env([("ZED", "1"), ("ALPHA", "2")])
        .build()
        .unwrap();
    let command = build_command(&config).unwrap();

    let envs: Vec<(String, String)> = command
        .get_envs()
        .map(|(key, value)| {
            (
                key.to_string_lossy().into_owned(),
                value.expect("set, not removed").to_string_lossy().into_owned(),
            )
        })
        .collect();
    assert_eq!(envs, [("ALPHA".to_string(), "2".to_string()), ("ZED".to_string(), "1".to_string())]);
}
