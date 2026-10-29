use super::*;

#[test]
fn the_profile_payload_maps_onto_the_profile() {
    // Recorded shape of `/minecraft/profile`.
    let payload = r#"{ "id": "b4beefcafe1234567890abcdef123456", "name": "Steve", "skins": [], "capes": [] }"#;
    let profile: ProfileResponse = serde_json::from_str(payload).expect("recorded payload parses");
    let profile = Profile {
        id: profile.id,
        name: profile.name,
        skin_url: active_skin_url(profile.skins),
    };
    assert_eq!(profile.id, "b4beefcafe1234567890abcdef123456");
    assert_eq!(profile.name, "Steve");
    assert_eq!(profile.skin_url, None, "no skins means no texture url");
}

#[test]
fn the_active_skin_url_wins_over_other_entries() {
    let payload = r#"{
        "id": "b4beefcafe1234567890abcdef123456",
        "name": "Steve",
        "skins": [
            { "id": "s1", "state": "INACTIVE", "url": "https://textures.minecraft.net/texture/old" },
            { "id": "s2", "state": "ACTIVE", "url": "https://textures.minecraft.net/texture/new" }
        ]
    }"#;
    let profile: ProfileResponse = serde_json::from_str(payload).expect("recorded payload parses");
    assert_eq!(
        active_skin_url(profile.skins).as_deref(),
        Some("https://textures.minecraft.net/texture/new")
    );
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
