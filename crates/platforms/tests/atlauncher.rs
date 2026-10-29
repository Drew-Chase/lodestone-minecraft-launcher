use super::*;

/// Recorded slice of the real `/v1/pack/SkyFactory` response.
const PACK: &str = r#"{
    "error": false,
    "code": 200,
    "message": null,
    "data": {
        "id": 190,
        "name": "Sky Factory",
        "safeName": "SkyFactory",
        "type": "public",
        "description": "A skyblock pack",
        "versions": [
            { "version": "2.4.1", "minecraft": "1.7.10", "published": 1639190283 },
            { "version": "2.4", "minecraft": "1.7.10", "published": 1639190283 }
        ]
    }
}"#;

#[test]
fn the_pack_envelope_parses() {
    let envelope: Envelope<Pack> = serde_json::from_str(PACK).expect("recorded payload parses");
    let pack = envelope.data.expect("data present");
    assert_eq!(pack.name, "Sky Factory");
    assert_eq!(pack.safe_name.as_deref(), Some("SkyFactory"));
    assert_eq!(pack.versions.len(), 2);
    assert_eq!(pack.versions[0].version, "2.4.1");
    assert_eq!(pack.versions[0].minecraft_version, "1.7.10");
}

#[test]
fn versions_map_with_their_minecraft_version() {
    let envelope: Envelope<Pack> = serde_json::from_str(PACK).expect("recorded payload parses");
    let pack = envelope.data.expect("data present");
    let versions: Vec<ContentVersion> = pack
        .versions
        .into_iter()
        .map(|version| ContentVersion {
            id: version.version.clone(),
            project_id: None,
            name: Some(version.version.clone()),
            version_number: version.version,
            content_type: ContentType::Modpack,
            minecraft_versions: vec![version.minecraft_version],
            loaders: Vec::new(),
            downloads: 0,
            date: version.published.map(|published| published.to_string()),
            changelog_url: None,
            files: Vec::new(),
        })
        .collect();
    assert_eq!(versions.len(), 2);
    assert_eq!(versions[0].minecraft_versions, ["1.7.10"]);
}
