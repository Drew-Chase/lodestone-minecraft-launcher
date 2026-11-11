use super::*;
use serde_json::json;
use std::io::Write;

/// A fresh folder for one test, under the system temporary folder, named after the test.
fn scratch(name: &str) -> PathBuf {
    let dir = std::env::temp_dir().join(format!("lodestone-loaders-forge-profile-{}-{name}", std::process::id()));
    let _ = std::fs::remove_dir_all(&dir);
    std::fs::create_dir_all(&dir).expect("the scratch folder creates");
    dir
}

/// An installer jar holding `entries`, written under `dir`.
fn installer(dir: &Path, entries: &[(&str, &[u8])]) -> PathBuf {
    let path = dir.join("forge-installer.jar");
    let mut zip = zip::ZipWriter::new(std::fs::File::create(&path).expect("the jar creates"));
    for (name, bytes) in entries {
        zip.start_file(*name, zip::write::SimpleFileOptions::default()).expect("the entry starts");
        zip.write_all(bytes).expect("the entry writes");
    }
    zip.finish().expect("the jar finishes");
    path
}

/// A `versionInfo` profile for Minecraft 1.5.2 that names `libraries`.
fn profile(libraries: Value) -> Value {
    json!({
        "install": {
            "target": "1.5.2-Forge7.8.1.738",
            "path": "net.minecraftforge:minecraftforge:7.8.1.738",
            "filePath": "minecraftforge-universal-1.5.2-7.8.1.738.jar",
            "minecraft": "1.5.2"
        },
        "versionInfo": {
            "id": "1.5.2-Forge7.8.1.738",
            "mainClass": "net.minecraft.launchwrapper.Launch",
            "minecraftArguments": "--username ${auth_player_name}",
            "libraries": libraries
        }
    })
}

/// The universal jar's entry name in [`profile`].
const UNIVERSAL: &str = "minecraftforge-universal-1.5.2-7.8.1.738.jar";

#[tokio::test]
async fn a_profile_without_processors_is_the_version_info_layout() {
    let jar = installer(&scratch("layout-version-info"), &[(PROFILE, profile(json!([])).to_string().as_bytes())]);
    assert_eq!(layout(jar).await.expect("the layout reads"), Layout::VersionInfo);
}

#[tokio::test]
async fn a_profile_with_processors_or_no_profile_runs_the_installer() {
    let processors = json!({ "install": {}, "processors": [], "versionInfo": {} });
    let jar = installer(&scratch("layout-processors"), &[(PROFILE, processors.to_string().as_bytes())]);
    assert_eq!(layout(jar).await.expect("the layout reads"), Layout::Processors);

    let bare = installer(
        &scratch("layout-no-profile"),
        &[("META-INF/MANIFEST.MF", b"Manifest-Version: 1.0".as_slice())],
    );
    assert_eq!(layout(bare).await.expect("the layout reads"), Layout::Processors);
}

#[tokio::test]
async fn a_jar_that_is_not_a_zip_is_reported_by_its_path() {
    let dir = scratch("layout-not-a-zip");
    let jar = dir.join("forge-installer.jar");
    std::fs::write(&jar, b"not a zip").expect("the file writes");
    let error = layout(jar.clone()).await.expect_err("a file that is not a zip has no layout");
    assert!(
        matches!(&error, LoaderError::InstallerUnreadable { path, .. } if *path == jar),
        "{error:?}"
    );
}

#[tokio::test]
async fn the_universal_jar_and_the_version_json_land_where_the_launcher_reads_them() {
    let dir = scratch("install-universal");
    let jar = installer(
        &dir,
        &[(PROFILE, profile(json!([])).to_string().as_bytes()), (UNIVERSAL, b"universal".as_slice())],
    );
    let game = dir.join("game");

    let installed = install(&lodestone_net::client(), &jar, &game, None, None)
        .await
        .expect("the profile installs");

    assert_eq!(installed.id, "1.5.2-Forge7.8.1.738");
    assert_eq!(installed.json, game.join("versions/1.5.2-Forge7.8.1.738/1.5.2-Forge7.8.1.738.json"));
    let universal = game.join("libraries/net/minecraftforge/minecraftforge/7.8.1.738/minecraftforge-7.8.1.738.jar");
    assert_eq!(std::fs::read(&universal).expect("the universal jar is in libraries"), b"universal");
    let written: Value =
        serde_json::from_str(&std::fs::read_to_string(&installed.json).expect("the version JSON is written")).expect("the version JSON parses");
    assert_eq!(written, profile(json!([]))["versionInfo"]);
}

#[test]
fn only_the_libraries_marked_for_the_client_are_wanted() {
    let version_info = json!({ "libraries": [
        { "name": "org.example:client:1.0", "clientreq": true, "url": "https://maven.example/" },
        { "name": "org.example:server:1.0", "serverreq": true },
        { "name": "org.example:plain:1.0" },
        { "name": "org.example:both:1.0", "clientreq": true, "serverreq": true,
          "checksums": ["0123456789abcdef0123456789abcdef01234567"] }
    ]});

    let wanted = wanted_libraries(&version_info);

    let names: Vec<&str> = wanted.iter().map(|library| library.name.as_str()).collect();
    assert_eq!(names, ["org.example:client:1.0", "org.example:both:1.0"]);
    assert_eq!(wanted[0].url.as_deref(), Some("https://maven.example/"));
    assert_eq!(wanted[1].checksums, ["0123456789abcdef0123456789abcdef01234567"]);
}

#[tokio::test]
async fn a_library_the_network_refuses_comes_from_the_installer_instead() {
    let dir = scratch("install-embedded");
    let libraries = json!([{ "name": "org.example:thing:1.0", "clientreq": true, "url": "http://127.0.0.1:9/" }]);
    let jar = installer(
        &dir,
        &[
            (PROFILE, profile(libraries).to_string().as_bytes()),
            (UNIVERSAL, b"universal".as_slice()),
            ("maven/org/example/thing/1.0/thing-1.0.jar", b"embedded library".as_slice()),
        ],
    );
    let game = dir.join("game");

    install(&lodestone_net::client(), &jar, &game, None, None)
        .await
        .expect("the copy inside the installer stands in for the download");

    let library = game.join("libraries/org/example/thing/1.0/thing-1.0.jar");
    assert_eq!(std::fs::read(&library).expect("the library is in libraries"), b"embedded library");
}

#[tokio::test]
async fn a_damaged_library_is_replaced_by_one_that_matches_its_checksum() {
    let dir = scratch("install-checksum");
    let right = lodestone_net::download::sha1_hex(b"the right library");
    let libraries = json!([{
        "name": "org.example:thing:1.0",
        "clientreq": true,
        "url": "http://127.0.0.1:9/",
        "checksums": [right]
    }]);
    let jar = installer(
        &dir,
        &[
            (PROFILE, profile(libraries).to_string().as_bytes()),
            (UNIVERSAL, b"universal".as_slice()),
            ("maven/org/example/thing/1.0/thing-1.0.jar", b"the right library".as_slice()),
        ],
    );
    let game = dir.join("game");
    let library = game.join("libraries/org/example/thing/1.0/thing-1.0.jar");
    std::fs::create_dir_all(library.parent().expect("the library has a folder")).expect("the folder creates");
    std::fs::write(&library, b"a damaged library").expect("the damaged copy writes");

    install(&lodestone_net::client(), &jar, &game, None, None)
        .await
        .expect("the copy that matches replaces the damaged one");

    assert_eq!(std::fs::read(&library).expect("the library is in libraries"), b"the right library");
}

#[tokio::test]
async fn a_library_nothing_provides_fails_the_install() {
    let dir = scratch("install-missing");
    let libraries = json!([{ "name": "org.example:gone:1.0", "clientreq": true, "url": "http://127.0.0.1:9/" }]);
    let jar = installer(
        &dir,
        &[(PROFILE, profile(libraries).to_string().as_bytes()), (UNIVERSAL, b"universal".as_slice())],
    );

    let error = install(&lodestone_net::client(), &jar, &dir.join("game"), None, None)
        .await
        .expect_err("no source has the library");

    assert!(
        matches!(&error, LoaderError::Download { url, .. } if url.ends_with("org/example/gone/1.0/gone-1.0.jar")),
        "{error:?}"
    );
}
