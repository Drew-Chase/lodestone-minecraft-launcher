use super::*;

fn release(tag: &str, names: &[&str]) -> Release {
    Release {
        tag: tag.to_string(),
        html_url: format!("https://example.test/releases/{tag}"),
        assets: names
            .iter()
            .map(|name| Asset {
                name: name.to_string(),
                browser_download_url: format!("https://example.test/download/{name}"),
            })
            .collect(),
    }
}

#[test]
fn the_tag_without_its_v_is_the_version() {
    assert_eq!(release("v0.2.0", &[]).version().expect("a version"), Version::new(0, 2, 0));
    assert_eq!(release("0.2.0", &[]).version().expect("a version"), Version::new(0, 2, 0));
    assert!(release("nightly", &[]).version().is_err());
}

#[test]
fn only_a_newer_release_is_offered() {
    let current = Version::new(0, 1, 0);
    assert!(newer(&current, release("v0.2.0", &[])).expect("a comparison").is_some());
    assert!(newer(&current, release("v0.1.0", &[])).expect("a comparison").is_none());
    assert!(newer(&current, release("v0.0.9", &[])).expect("a comparison").is_none());
}

#[test]
fn the_installer_is_the_setup_asset() {
    let found = release(
        "v0.2.0",
        &["Lodestone-v0.2.0-linux-x86_64.tar.gz", "Lodestone-v0.2.0-setup.exe", "SHA256SUMS.txt"],
    );
    assert_eq!(
        installer_asset(&found).map(|asset| asset.name.as_str()),
        Some("Lodestone-v0.2.0-setup.exe")
    );
    assert!(installer_asset(&release("v0.2.0", &["SHA256SUMS.txt"])).is_none());
}

#[test]
fn a_release_reads_from_github_json() {
    let json = r#"{
        "tag_name": "v0.2.0",
        "html_url": "https://github.com/Drew-Chase/lodestone-minecraft-launcher/releases/tag/v0.2.0",
        "draft": false,
        "prerelease": false,
        "body": null,
        "assets": [
            {"name": "Lodestone-v0.2.0-setup.exe", "browser_download_url": "https://example.test/setup.exe", "size": 1}
        ]
    }"#;
    let parsed: Release = serde_json::from_str(json).expect("a release");
    assert_eq!(parsed.tag, "v0.2.0");
    assert_eq!(parsed.assets.len(), 1);
    assert_eq!(parsed.version().expect("a version"), Version::new(0, 2, 0));
}
