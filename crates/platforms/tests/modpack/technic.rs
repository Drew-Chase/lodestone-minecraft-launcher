use super::*;
use std::io::Write;
use zip::write::SimpleFileOptions;

/// The `version.json` a Forge 1.12.2 pack ships in `bin/`.
const FORGE: &str = r#"{
    "id": "1.12.2-forge1.12.2-14.23.5.2847",
    "inheritsFrom": "1.12.2",
    "libraries": [
        { "name": "org.ow2.asm:asm-all:5.2" },
        { "name": "net.minecraftforge:forge:1.12.2-14.23.5.2847" }
    ]
}"#;

/// The `version.json` of a Fabric pack.
const FABRIC: &str = r#"{
    "id": "fabric-loader-0.14.21-1.20.1",
    "libraries": [
        { "name": "net.fabricmc:fabric-loader:0.14.21" }
    ]
}"#;

/// The bytes of a zip holding the given entries, as a pack ships them.
fn zip_of(entries: &[(&str, &[u8])]) -> Vec<u8> {
    let mut writer = zip::ZipWriter::new(Cursor::new(Vec::new()));
    for (name, bytes) in entries {
        writer.start_file(*name, SimpleFileOptions::default()).expect("entry starts");
        writer.write_all(bytes).expect("entry writes");
    }
    writer.finish().expect("zip finishes").into_inner()
}

fn read(bytes: Vec<u8>) -> Result<Modpack, PlatformError> {
    read_pack(Cursor::new(bytes), Path::new("Hexxit.zip"))
}

#[test]
fn a_zip_names_its_loader_from_its_version_json() {
    let modpack = read(zip_of(&[("bin/version.json", FORGE.as_bytes()), ("mods/a.jar", "jar".as_bytes())])).expect("pack reads");
    assert_eq!(modpack.name, "Hexxit");
    assert_eq!(modpack.loader, Some("forge"));
    assert_eq!(modpack.loader_version.as_deref(), Some("1.12.2-14.23.5.2847"));
    assert_eq!(modpack.minecraft, "1.12.2");
    assert_eq!(modpack.overrides.as_deref(), Some(""), "the zip unpacks from the instance root");
    assert_eq!(modpack.overrides_skip, ["bin/modpack.jar", "bin/version.json"]);
    assert!(modpack.files.is_empty(), "the zip itself carries the mods");
}

#[test]
fn the_descriptor_inside_modpack_jar_is_found() {
    let jar = zip_of(&[("version.json", FORGE.as_bytes())]);
    let modpack = read(zip_of(&[("bin/modpack.jar", jar.as_slice())])).expect("pack reads");
    assert_eq!(modpack.loader, Some("forge"));
    assert_eq!(modpack.loader_version.as_deref(), Some("1.12.2-14.23.5.2847"));
}

#[test]
fn a_fabric_pack_names_fabric() {
    let modpack = read(zip_of(&[("bin/version.json", FABRIC.as_bytes())])).expect("pack reads");
    assert_eq!(modpack.loader, Some("fabric"));
    assert_eq!(modpack.loader_version.as_deref(), Some("0.14.21"));
    assert!(modpack.minecraft.is_empty(), "a Fabric descriptor names no Minecraft version");
}

#[test]
fn a_zip_without_a_descriptor_is_vanilla() {
    let modpack = read(zip_of(&[("mods/a.jar", "jar".as_bytes())])).expect("pack reads");
    assert_eq!(modpack.loader, None);
    assert_eq!(modpack.loader_version, None);
}

#[test]
fn a_file_that_is_not_a_zip_is_a_parse_error() {
    let error = read(b"not a zip".to_vec()).expect_err("not a zip");
    assert!(matches!(error, PlatformError::ModpackParse { .. }));
}

#[test]
fn a_descriptor_that_is_not_json_is_a_parse_error() {
    let error = read(zip_of(&[("bin/version.json", "{".as_bytes())])).expect_err("bad descriptor");
    assert!(matches!(error, PlatformError::ModpackParse { .. }));
}
