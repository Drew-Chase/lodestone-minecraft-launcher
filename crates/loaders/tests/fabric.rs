use super::*;

/// Recorded shape from `https://meta.fabricmc.net/v2/versions/loader/1.21.1`.
const SAMPLE_ENTRY: &str = r#"{
    "loader": { "version": "0.16.14", "stable": true },
    "intermediary": { "version": "0.14.2+1.21.1", "stable": true },
    "launcherMeta": {
        "version": "2",
        "min_launcher_version": 2,
        "libraries": {
            "client": [{ "name": "org.ow2.asm:asm:9.7.1", "url": "https://maven.fabricmc.net/" }],
            "common": [
                { "name": "net.fabricmc:fabric-loader:0.16.14", "url": "https://maven.fabricmc.net/" },
                { "name": "net.fabricmc:sponge-mixin:0.12.5+mixin.0.8.5" }
            ]
        }
    }
}"#;

#[test]
fn entries_parse_from_the_real_meta_shape() {
    let entry: Entry = serde_json::from_str(SAMPLE_ENTRY).expect("sample is a real meta entry");
    assert_eq!(entry.loader.version, "0.16.14");
    assert!(entry.loader.stable);
    assert_eq!(entry.launcher_meta.libraries.client.len(), 1);
    assert_eq!(entry.launcher_meta.libraries.common.len(), 2);
    assert_eq!(entry.launcher_meta.libraries.common[0].name, "net.fabricmc:fabric-loader:0.16.14");
    assert_eq!(entry.launcher_meta.libraries.common[1].url, None);
}

#[test]
fn download_urls_follow_per_artifact_bases() {
    let entry: Entry = serde_json::from_str(SAMPLE_ENTRY).expect("sample is a real meta entry");
    let files: Vec<(String, PathBuf)> = entry
        .launcher_meta
        .libraries
        .client
        .iter()
        .chain(&entry.launcher_meta.libraries.common)
        .filter_map(|artifact| {
            let relative = maven_jar_path(&artifact.name)?;
            let base = maven_base(artifact.url.as_deref().unwrap_or(DEFAULT_MAVEN));
            Some((format!("{base}{relative}"), PathBuf::from(&relative)))
        })
        .collect();

    let urls: Vec<&str> = files.iter().map(|(url, _)| url.as_str()).collect();
    assert_eq!(
        urls,
        [
            "https://maven.fabricmc.net/org/ow2/asm/asm/9.7.1/asm-9.7.1.jar",
            "https://maven.fabricmc.net/net/fabricmc/fabric-loader/0.16.14/fabric-loader-0.16.14.jar",
            "https://maven.fabricmc.net/net/fabricmc/sponge-mixin/0.12.5+mixin.0.8.5/sponge-mixin-0.12.5+mixin.0.8.5.jar",
        ]
    );
}
