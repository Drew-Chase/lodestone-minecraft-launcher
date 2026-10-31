//! The public platform API over fixture archives — no network in CI
//! (DESIGN.md §9). Recorded-payload parsing tests live inside the kind modules
//! as unit tests, since the serde structs are `pub(crate)`.

mod fixture;

use fixture::jar;
use lodestone_platforms::{AtLauncher, ContentType, CurseForge, Ftb, Modrinth, Platform, PlatformError, SearchQuery, Technic};

/// A first page of `content` matching `text`.
fn query(text: &str, content: ContentType) -> SearchQuery {
    SearchQuery {
        text: text.to_string(),
        ..SearchQuery::new(content)
    }
}

const MRPACK_INDEX: &str = r#"{
    "formatVersion": 1,
    "game": "minecraft",
    "versionId": "1.2.3",
    "name": "Test Pack",
    "dependencies": { "minecraft": "1.21.1", "fabric-loader": "0.16.14" },
    "files": [
        {
            "path": "mods/sodium.jar",
            "hashes": { "sha1": "abc" },
            "downloads": ["https://cdn.modrinth.com/x.jar"],
            "fileSize": 1234
        }
    ],
    "overrides": "overrides"
}"#;

const CURSEFORGE_MANIFEST: &str = r#"{
    "manifestType": "minecraftModpack",
    "manifestVersion": 1,
    "name": "CF Pack",
    "version": "2.0.0",
    "minecraft": { "version": "1.20.1", "modLoaders": [ { "id": "forge-47.4.0", "primary": true } ] },
    "files": [ { "projectID": 306612, "fileID": 4838285, "required": true } ],
    "overrides": "overrides"
}"#;

#[tokio::test]
async fn capability_matrix_gates_content_types() {
    let platform = Platform::<AtLauncher>::new();
    let error = platform.search(&query("anything", ContentType::Mod)).await.expect_err("out of matrix");
    assert!(matches!(
        error,
        PlatformError::UnsupportedContentType {
            platform: "atlauncher",
            content: ContentType::Mod
        }
    ));

    let platform = Platform::<CurseForge>::new();
    let error = platform.search(&query("anything", ContentType::Shader)).await.expect_err("out of matrix");
    assert!(matches!(
        error,
        PlatformError::UnsupportedContentType {
            platform: "curseforge",
            content: ContentType::Shader
        }
    ));

    let error = platform.details("1", ContentType::DataPack).await.expect_err("out of matrix");
    assert!(matches!(
        error,
        PlatformError::UnsupportedContentType {
            platform: "curseforge",
            content: ContentType::DataPack
        }
    ));
}

#[test]
fn pack_only_platforms_reject_modpack_archive_parsing() {
    let path = jar("plain", &[("some.txt", "not a modpack")]);
    for result in [
        Platform::<AtLauncher>::new().parse_modpack(&path),
        Platform::<Ftb>::new().parse_modpack(&path),
    ] {
        assert!(matches!(
            result,
            Err(PlatformError::UnsupportedPlatform { platform: "atlauncher", .. }) | Err(PlatformError::UnsupportedPlatform { platform: "ftb", .. })
        ));
    }
}

#[test]
fn technic_reads_a_zip_without_a_loader_descriptor_as_vanilla() {
    let path = jar("plain", &[("some.txt", "not a modpack")]);
    let modpack = Platform::<Technic>::new().parse_modpack(&path).expect("a zip reads");
    assert_eq!(modpack.loader, None);
}

#[test]
fn mrpack_archives_parse_through_the_modrinth_kind() {
    let path = jar("mrpack", &[("modrinth.index.json", MRPACK_INDEX), ("overrides/config.toml", "x = 1")]);
    let modpack = Platform::<Modrinth>::new().parse_modpack(&path).expect("mrpack parses");

    assert_eq!(modpack.name, "Test Pack");
    assert_eq!(modpack.version.as_deref(), Some("1.2.3"));
    assert_eq!(modpack.minecraft, "1.21.1");
    assert_eq!(modpack.loader, Some("fabric"));
    assert_eq!(modpack.loader_version.as_deref(), Some("0.16.14"));
    assert_eq!(modpack.overrides.as_deref(), Some("overrides"));
    assert_eq!(modpack.files.len(), 1);
    assert_eq!(modpack.files[0].path, "mods/sodium.jar");
    assert_eq!(modpack.files[0].url, "https://cdn.modrinth.com/x.jar");
    assert_eq!(modpack.files[0].sha1.as_deref(), Some("abc"));
}

#[test]
fn curseforge_exports_parse_through_the_curseforge_kind() {
    let path = jar("cf-export", &[("manifest.json", CURSEFORGE_MANIFEST), ("overrides/options.txt", "y = 2")]);
    let modpack = Platform::<CurseForge>::new().parse_modpack(&path).expect("manifest parses");

    assert_eq!(modpack.name, "CF Pack");
    assert_eq!(modpack.minecraft, "1.20.1");
    assert_eq!(modpack.loader, Some("forge"));
    assert_eq!(modpack.loader_version.as_deref(), Some("47.4.0"));
    assert_eq!(modpack.files.len(), 1);
    assert_eq!(modpack.files[0].project_id.as_deref(), Some("306612"));
    assert_eq!(modpack.files[0].file_id.as_deref(), Some("4838285"));
    assert!(modpack.files[0].url.is_empty(), "curseforge ids resolve through the API at install time");
}

#[test]
fn unrecognized_archives_report_an_unknown_format() {
    let path = jar("unknown", &[("readme.txt", "hello")]);
    let result = Platform::<Modrinth>::new().parse_modpack(&path);
    assert!(matches!(result, Err(PlatformError::ModpackFormatUnknown { .. })));
}

#[test]
fn curseforge_without_a_key_fails_fast() {
    // A keyless CurseForge handle may not even attempt a network call: the
    // key check happens before the request.
    let platform = Platform::<CurseForge>::new();
    let sodium = query("sodium", ContentType::Mod);
    let result = platform.search(&sodium);
    // The search is async; the key check fires inside it, so drive it to
    // completion on the current thread runtime.
    let runtime = tokio::runtime::Builder::new_current_thread().build().expect("runtime builds");
    let error = runtime.block_on(result).expect_err("keyless curseforge fails");
    assert!(matches!(error, PlatformError::UnexpectedResponse { status: 401, .. }));
}
