use super::*;

#[test]
fn the_profile_payload_maps_onto_the_profile() {
    // Recorded shape of `/minecraft/profile` for a player with no skin and no capes.
    let payload = r#"{ "id": "b4beefcafe1234567890abcdef123456", "name": "Steve", "skins": [], "capes": [] }"#;
    let profile = serde_json::from_str::<ProfileResponse>(payload)
        .expect("recorded payload parses")
        .into_profile();
    assert_eq!(profile.id, "b4beefcafe1234567890abcdef123456");
    assert_eq!(profile.name, "Steve");
    assert_eq!(profile.skin_url, None, "no skins means no texture url");
    assert!(!profile.slim);
    assert!(profile.capes.is_empty());
}

#[test]
fn the_active_skin_and_worn_cape_win_over_other_entries() {
    let payload = r#"{
        "id": "b4beefcafe1234567890abcdef123456",
        "name": "Steve",
        "skins": [
            { "id": "s1", "state": "INACTIVE", "url": "https://textures.minecraft.net/texture/old", "variant": "CLASSIC" },
            { "id": "s2", "state": "ACTIVE", "url": "https://textures.minecraft.net/texture/new", "variant": "SLIM" }
        ],
        "capes": [
            { "id": "c1", "state": "INACTIVE", "url": "https://textures.minecraft.net/texture/cape-a", "alias": "Migrator" },
            { "id": "c2", "state": "ACTIVE", "url": "https://textures.minecraft.net/texture/cape-b", "alias": "MINECON 2016" }
        ]
    }"#;
    let profile = serde_json::from_str::<ProfileResponse>(payload)
        .expect("recorded payload parses")
        .into_profile();
    assert_eq!(profile.skin_url.as_deref(), Some("https://textures.minecraft.net/texture/new"));
    assert!(profile.slim, "the active skin is the slim model");
    assert_eq!(
        profile.capes,
        vec![
            Cape {
                id: "c1".to_string(),
                name: "Migrator".to_string(),
                url: "https://textures.minecraft.net/texture/cape-a".to_string(),
                active: false,
            },
            Cape {
                id: "c2".to_string(),
                name: "MINECON 2016".to_string(),
                url: "https://textures.minecraft.net/texture/cape-b".to_string(),
                active: true,
            },
        ]
    );
}

#[test]
fn null_lists_read_as_empty_and_a_classic_skin_is_not_slim() {
    let payload = r#"{ "id": "b4beefcafe1234567890abcdef123456", "name": "Steve", "skins": null, "capes": null }"#;
    let profile = serde_json::from_str::<ProfileResponse>(payload)
        .expect("recorded payload parses")
        .into_profile();
    assert_eq!(profile.skin_url, None);
    assert!(profile.capes.is_empty());
    let classic = r#"{ "id": "x", "name": "Alex", "skins": [{ "state": "ACTIVE", "url": "u", "variant": "CLASSIC" }] }"#;
    let profile = serde_json::from_str::<ProfileResponse>(classic)
        .expect("recorded payload parses")
        .into_profile();
    assert!(!profile.slim);
}

#[test]
fn the_skin_upload_body_names_the_model_and_carries_the_png() {
    let body = String::from_utf8_lossy(&upload::skin_upload_body(true, b"PNGDATA")).into_owned();
    assert!(body.contains("name=\"variant\"\r\n\r\nslim\r\n"), "the model is a form field");
    assert!(body.contains("filename=\"skin.png\""));
    assert!(body.contains("PNGDATA"));
    assert!(body.ends_with(&format!("--{}--\r\n", upload::BOUNDARY)));
}

#[test]
fn the_cape_and_skin_endpoints_target_the_profile() {
    assert!(endpoints::CAPE_ACTIVE_URL.ends_with("/minecraft/profile/capes/active"));
    assert!(endpoints::SKIN_UPLOAD_URL.ends_with("/minecraft/profile/skins"));
}

#[test]
fn instance_discovery_regions_are_extracted_when_reported() {
    // Recorded shape of `/common/discovery/instance` (AAD tenants).
    let body = r#"{
        "tenant_region_scope": "NA",
        "tenant_region_sub_scope": "EASTUS",
        "cloud_instance_name": "microsoftonline.com",
        "api-version": "1.1"
    }"#;
    let (scope, sub_scope) = parse_instance_discovery(body);
    assert_eq!(scope.as_deref(), Some("NA"));
    assert_eq!(sub_scope.as_deref(), Some("EASTUS"));
}

#[test]
fn instance_discovery_tolerates_missing_and_blank_regions() {
    // Personal (MSA) accounts frequently report neither scope.
    let body = r#"{ "tenant_region_scope": "", "cloud_instance_name": "microsoftonline.com" }"#;
    let (scope, sub_scope) = parse_instance_discovery(body);
    assert_eq!(scope, None);
    assert_eq!(sub_scope, None);
    let (scope, sub_scope) = parse_instance_discovery("not json at all");
    assert_eq!((scope, sub_scope), (None, None));
}

#[test]
fn every_msa_endpoint_targets_the_consumers_tenant() {
    // The app registration is "Personal Microsoft Account users only" —
    // /common rejects its token and device-code requests with
    // AADSTS9002331, so every OAuth endpoint must pin /consumers.
    for url in [endpoints::MSA_AUTHORIZE_URL, endpoints::MSA_TOKEN_URL, endpoints::DEVICE_CODE_URL] {
        assert!(url.contains("/consumers/"), "{url} must target /consumers");
    }
}

#[test]
fn session_expiry_takes_the_earlier_lifetime() {
    let before = unix_now();
    let expiry = session_expiry(86_400, 3_600);
    assert!(expiry >= before + 3_600, "at least the microsoft lifetime");
    assert!(expiry < before + 86_400, "never longer than the shorter lifetime");
}
