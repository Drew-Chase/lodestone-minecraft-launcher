use super::*;

const VANILLA: &str = r#"{
  "id": "1.21.4",
  "type": "release",
  "mainClass": "net.minecraft.client.main.Main",
  "assetIndex": {"id": "17", "url": "https://piston-meta.example/17.json", "sha1": "aa", "size": 100, "totalSize": 200},
  "assets": "17",
  "downloads": {"client": {"sha1": "bb", "size": 27000000, "url": "https://data.example/client.jar"}},
  "javaVersion": {"component": "java-runtime-delta", "majorVersion": 21},
  "logging": {"client": {"argument": "-Dlog4j.configurationFile=${path}", "type": "log4j2-xml",
    "file": {"id": "client-1.12.xml", "sha1": "cc", "size": 900, "url": "https://data.example/client-1.12.xml"}}},
  "arguments": {
    "game": ["--username", "${auth_player_name}", {"rules": [{"action": "allow", "features": {"has_custom_resolution": true}}], "value": ["--width", "${resolution_width}"]}],
    "jvm": ["-Djava.library.path=${natives_directory}", "-cp", "${classpath}"]
  },
  "libraries": [
    {"name": "org.lwjgl:lwjgl:3.3.3", "downloads": {"artifact": {"path": "org/lwjgl/lwjgl/3.3.3/lwjgl-3.3.3.jar", "sha1": "dd", "size": 10, "url": "https://libraries.example/lwjgl.jar"}}},
    {"name": "org.ow2.asm:asm:9.6", "downloads": {"artifact": {"path": "org/ow2/asm/asm/9.6/asm-9.6.jar", "sha1": "ee", "size": 20, "url": "https://libraries.example/asm-9.6.jar"}}}
  ]
}"#;

const FABRIC: &str = r#"{
  "id": "fabric-loader-0.16.10-1.21.4",
  "inheritsFrom": "1.21.4",
  "mainClass": "net.fabricmc.loader.impl.launch.knot.KnotClient",
  "libraries": [
    {"name": "org.ow2.asm:asm:9.7", "url": "https://maven.fabricmc.net/"},
    {"name": "net.fabricmc:fabric-loader:0.16.10", "url": "https://maven.fabricmc.net/"}
  ]
}"#;

#[test]
fn a_vanilla_version_reads_its_downloads_index_and_logging() {
    let manifest = Manifest::parse(VANILLA).expect("the vanilla JSON parses");
    assert_eq!(manifest.id, "1.21.4");
    assert_eq!(manifest.kind.as_deref(), Some("release"));
    assert_eq!(manifest.main_class.as_deref(), Some("net.minecraft.client.main.Main"));
    let client = manifest
        .downloads
        .and_then(|downloads| downloads.client)
        .expect("the client jar is listed");
    assert_eq!(client.url, "https://data.example/client.jar");
    assert_eq!(client.size, Some(27_000_000));
    let index = manifest.asset_index.expect("the asset index is listed");
    assert_eq!(index.id, "17");
    let java = manifest.java_version.expect("the Java component is listed");
    assert_eq!(java.component, "java-runtime-delta");
    let logging = manifest.logging.expect("the logging config is listed");
    assert_eq!(logging.client.file.id, "client-1.12.xml");
    assert_eq!(logging.client.argument, "-Dlog4j.configurationFile=${path}");
}

#[test]
fn conditional_arguments_keep_their_rules_and_values() {
    let manifest = Manifest::parse(VANILLA).expect("the vanilla JSON parses");
    let game = manifest.arguments.expect("arguments are listed").game;
    assert!(matches!(game[0], Argument::Plain(ref text) if text == "--username"));
    match &game[2] {
        Argument::Conditional { rules, value } => {
            assert_eq!(rules.len(), 1);
            assert_eq!(value.strings(), vec!["--width".to_string(), "${resolution_width}".to_string()]);
        }
        other => panic!("expected a conditional argument, got {other:?}"),
    }
}

#[test]
fn a_loader_profile_folds_over_its_vanilla_version() {
    let vanilla = Manifest::parse(VANILLA).expect("the vanilla JSON parses");
    let profile = Manifest::parse(FABRIC).expect("the Fabric profile parses");
    let merged = profile.over(vanilla);

    assert_eq!(merged.id, "fabric-loader-0.16.10-1.21.4");
    assert_eq!(merged.main_class.as_deref(), Some("net.fabricmc.loader.impl.launch.knot.KnotClient"));
    assert!(
        merged.downloads.and_then(|downloads| downloads.client).is_some(),
        "the vanilla client jar is inherited"
    );
    assert_eq!(merged.asset_index.map(|index| index.id), Some("17".to_string()));
    assert_eq!(merged.java_version.map(|java| java.component), Some("java-runtime-delta".to_string()));

    let libraries: Vec<&str> = merged.libraries.iter().map(|library| library.name.as_str()).collect();
    assert_eq!(
        libraries,
        vec!["org.ow2.asm:asm:9.7", "net.fabricmc:fabric-loader:0.16.10", "org.lwjgl:lwjgl:3.3.3"],
        "the loader's asm replaces the vanilla one, and the loader's libraries come first"
    );
}

#[test]
fn a_legacy_version_keeps_its_argument_string() {
    let manifest = Manifest::parse(r#"{"id":"1.7.10","minecraftArguments":"--username ${auth_player_name} --version ${version_name}"}"#)
        .expect("the legacy JSON parses");
    assert_eq!(
        manifest.minecraft_arguments.as_deref(),
        Some("--username ${auth_player_name} --version ${version_name}")
    );
    assert!(manifest.arguments.is_none());
}

#[test]
fn a_library_key_drops_the_version_and_keeps_the_classifier() {
    assert_eq!(library_key("org.lwjgl:lwjgl:3.3.3"), "org.lwjgl:lwjgl");
    assert_eq!(library_key("org.lwjgl:lwjgl:3.3.3:natives-windows"), "org.lwjgl:lwjgl:natives-windows");
    assert_ne!(library_key("org.lwjgl:lwjgl:3.3.3"), library_key("org.lwjgl:lwjgl:3.3.3:natives-windows"));
}
