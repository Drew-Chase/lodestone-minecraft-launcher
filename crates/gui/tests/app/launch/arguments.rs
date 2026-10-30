use std::path::Path;

use super::*;
use crate::app::launch::identity::{Identity, offline_uuid};
use crate::app::launch::manifest::Manifest;
use crate::app::launch::rules::Platform;

const WINDOWS: Platform = Platform {
    os: "windows",
    arch: "x86_64",
};

fn values() -> Values<'static> {
    Values {
        version_name: "1.21.4",
        version_type: "release",
        game_directory: Path::new("/data/instances/Test"),
        assets_root: Path::new("/data/meta/assets"),
        assets_index_name: "17",
        library_directory: Path::new("/data/meta/libraries"),
        natives_directory: Path::new("/data/meta/natives/1.21.4"),
        classpath: "/data/meta/libraries/a.jar;/data/meta/versions/1.21.4/1.21.4.jar",
        launcher_name: "Lodestone Launcher",
        launcher_version: "0.1.0",
        client_id: "client-id",
    }
}

const VANILLA: &str = r#"{"id":"1.21.4","arguments":{
  "jvm":[
    "-Djava.library.path=${natives_directory}",
    "-Dminecraft.launcher.brand=${launcher_name}",
    "-cp",
    "${classpath}",
    {"rules":[{"action":"allow","os":{"name":"windows"}}],"value":"-XX:HeapDumpPath=MojangTricks"},
    {"rules":[{"action":"allow","os":{"name":"osx"}}],"value":"-XstartOnFirstThread"},
    "-Dunknown=${not_a_placeholder}"
  ],
  "game":[
    "--username","${auth_player_name}",
    "--uuid","${auth_uuid}",
    "--accessToken","${auth_access_token}",
    "--userType","${user_type}",
    "--version","${version_name}",
    "--gameDir","${game_directory}",
    "--assetsDir","${assets_root}",
    "--assetIndex","${assets_index_name}",
    "--versionType","${version_type}",
    "--clientId","${clientid}",
    "--xuid","${auth_xuid}",
    {"rules":[{"action":"allow","features":{"has_custom_resolution":true}}],"value":["--width","${resolution_width}"]}
  ]
}}"#;

#[test]
fn the_jvm_list_keeps_the_version_flags_and_drops_what_the_launcher_supplies() {
    let manifest = Manifest::parse(VANILLA).expect("the fixture parses");
    let identity = Identity::offline("Steve");
    let jvm = jvm(&manifest, &values(), &identity, WINDOWS);
    assert_eq!(
        jvm,
        vec![
            "-Dminecraft.launcher.brand=Lodestone Launcher".to_string(),
            "-XX:HeapDumpPath=MojangTricks".to_string(),
        ],
        "the classpath pair and the natives path are the launcher's; the macOS flag does not apply; the unknown placeholder is dropped"
    );
}

#[test]
fn an_offline_player_starts_as_a_legacy_user_with_token_zero() {
    let manifest = Manifest::parse(VANILLA).expect("the fixture parses");
    let identity = Identity::offline("Steve");
    let game = game(&manifest, &values(), &identity, WINDOWS);
    let uuid = offline_uuid("Steve");
    assert_eq!(
        game,
        vec![
            "--username",
            "Steve",
            "--uuid",
            uuid.as_str(),
            "--accessToken",
            "0",
            "--userType",
            "legacy",
            "--version",
            "1.21.4",
            "--gameDir",
            "/data/instances/Test",
            "--assetsDir",
            "/data/meta/assets",
            "--assetIndex",
            "17",
            "--versionType",
            "release",
            "--clientId",
            "client-id",
            "--xuid",
            "0",
        ]
        .into_iter()
        .map(str::to_string)
        .collect::<Vec<_>>()
    );
}

#[test]
fn a_microsoft_player_starts_with_the_profile_and_its_token() {
    let manifest = Manifest::parse(VANILLA).expect("the fixture parses");
    let identity = Identity::microsoft("Alex", "b4ccccad-1111-2222-3333-444455556666", "eyJ.token");
    let game = game(&manifest, &values(), &identity, WINDOWS);
    assert_eq!(
        &game[0..6],
        [
            "--username",
            "Alex",
            "--uuid",
            "b4ccccad111122223333444455556666",
            "--accessToken",
            "eyJ.token"
        ]
    );
    assert_eq!(&game[6..8], ["--userType", "msa"]);
}

#[test]
fn a_legacy_argument_string_is_split_and_filled_in() {
    let manifest = Manifest::parse(r#"{"id":"1.7.10","minecraftArguments":"--username ${auth_player_name} --version ${version_name}"}"#)
        .expect("the fixture parses");
    let identity = Identity::offline("Steve");
    assert_eq!(
        game(&manifest, &values(), &identity, WINDOWS),
        vec!["--username", "Steve", "--version", "1.21.4"]
    );
}

#[test]
fn the_logging_argument_points_at_its_config() {
    let manifest = Manifest::parse(
        r#"{"id":"1.21.4","logging":{"client":{"argument":"-Dlog4j.configurationFile=${path}","file":{"id":"client-1.12.xml","url":"https://x.example/c.xml"}}}}"#,
    )
    .expect("the fixture parses");
    assert_eq!(
        logging(&manifest, Path::new("/data/meta/log_configs/client-1.12.xml")).as_deref(),
        Some("-Dlog4j.configurationFile=/data/meta/log_configs/client-1.12.xml")
    );
    assert_eq!(logging(&Manifest::default(), Path::new("/x")), None);
}
