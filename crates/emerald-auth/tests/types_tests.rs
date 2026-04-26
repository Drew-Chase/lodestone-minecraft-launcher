use emerald_auth::minecraft::ProfileResponse;
use emerald_auth::types::{SkinVariant};

#[test]
fn profile_response_deserializes_full_profile() {
    let json = serde_json::json!({
        "id": "069a79f444e94726a5befca90e38aaf5",
        "name": "Notch",
        "skins": [
            {
                "id": "skin-id-1",
                "url": "https://textures.minecraft.net/texture/abc123",
                "variant": "CLASSIC"
            }
        ],
        "capes": [
            {
                "id": "cape-id-1",
                "url": "https://textures.minecraft.net/texture/cape456",
                "alias": "Minecon2016"
            }
        ]
    });

    let profile: ProfileResponse = serde_json::from_value(json).unwrap();
    assert_eq!(profile.id, "069a79f444e94726a5befca90e38aaf5");
    assert_eq!(profile.name, "Notch");
    assert_eq!(profile.skins.len(), 1);
    assert_eq!(profile.capes.len(), 1);
}

#[test]
fn profile_response_deserializes_without_skins_or_capes() {
    let json = serde_json::json!({
        "id": "069a79f444e94726a5befca90e38aaf5",
        "name": "Notch"
    });

    let profile: ProfileResponse = serde_json::from_value(json).unwrap();
    assert_eq!(profile.name, "Notch");
    assert!(profile.skins.is_empty());
    assert!(profile.capes.is_empty());
}

#[test]
fn active_skin_returns_none_when_empty() {
    let json = serde_json::json!({
        "id": "abc",
        "name": "Test",
        "skins": [],
        "capes": []
    });

    let profile: ProfileResponse = serde_json::from_value(json).unwrap();
    assert!(profile.active_skin().is_none());
    assert!(profile.active_cape().is_none());
}

#[test]
fn active_skin_parses_classic_variant() {
    let json = serde_json::json!({
        "id": "abc",
        "name": "Test",
        "skins": [{
            "id": "skin-1",
            "url": "https://example.com/skin.png",
            "variant": "CLASSIC"
        }],
        "capes": []
    });

    let profile: ProfileResponse = serde_json::from_value(json).unwrap();
    let skin = profile.active_skin().unwrap();
    assert_eq!(skin.id, "skin-1");
    assert!(matches!(skin.variant, SkinVariant::Classic));
}

#[test]
fn active_skin_parses_slim_variant() {
    let json = serde_json::json!({
        "id": "abc",
        "name": "Test",
        "skins": [{
            "id": "skin-2",
            "url": "https://example.com/skin.png",
            "variant": "SLIM"
        }],
        "capes": []
    });

    let profile: ProfileResponse = serde_json::from_value(json).unwrap();
    let skin = profile.active_skin().unwrap();
    assert!(matches!(skin.variant, SkinVariant::Slim));
}

#[test]
fn active_skin_defaults_unknown_variant_to_classic() {
    let json = serde_json::json!({
        "id": "abc",
        "name": "Test",
        "skins": [{
            "id": "skin-3",
            "url": "https://example.com/skin.png",
            "variant": "WIDE"
        }],
        "capes": []
    });

    let profile: ProfileResponse = serde_json::from_value(json).unwrap();
    let skin = profile.active_skin().unwrap();
    assert!(matches!(skin.variant, SkinVariant::Classic));
}

#[test]
fn active_skin_handles_lowercase_variant() {
    let json = serde_json::json!({
        "id": "abc",
        "name": "Test",
        "skins": [{
            "id": "skin-4",
            "url": "https://example.com/skin.png",
            "variant": "slim"
        }],
        "capes": []
    });

    let profile: ProfileResponse = serde_json::from_value(json).unwrap();
    let skin = profile.active_skin().unwrap();
    assert!(matches!(skin.variant, SkinVariant::Slim));
}

#[test]
fn active_cape_extracts_first_cape() {
    let json = serde_json::json!({
        "id": "abc",
        "name": "Test",
        "skins": [],
        "capes": [
            {
                "id": "cape-1",
                "url": "https://example.com/cape1.png",
                "alias": "Minecon2016"
            },
            {
                "id": "cape-2",
                "url": "https://example.com/cape2.png",
                "alias": "Minecon2019"
            }
        ]
    });

    let profile: ProfileResponse = serde_json::from_value(json).unwrap();
    let cape = profile.active_cape().unwrap();
    assert_eq!(cape.id, "cape-1");
    assert_eq!(cape.alias, "Minecon2016");
}

#[test]
fn skin_variant_serializes_to_uppercase() {
    let classic = serde_json::to_string(&SkinVariant::Classic).unwrap();
    let slim = serde_json::to_string(&SkinVariant::Slim).unwrap();
    assert_eq!(classic, "\"CLASSIC\"");
    assert_eq!(slim, "\"SLIM\"");
}

#[test]
fn skin_struct_roundtrips_through_serde() {
    let skin = emerald_auth::Skin {
        id: "skin-id".into(),
        url: "https://example.com/skin.png".into(),
        variant: SkinVariant::Slim,
    };
    let json = serde_json::to_string(&skin).unwrap();
    let deserialized: emerald_auth::Skin = serde_json::from_str(&json).unwrap();
    assert_eq!(deserialized.id, "skin-id");
    assert!(matches!(deserialized.variant, SkinVariant::Slim));
}

#[test]
fn cape_struct_roundtrips_through_serde() {
    let cape = emerald_auth::Cape {
        id: "cape-id".into(),
        url: "https://example.com/cape.png".into(),
        alias: "TestCape".into(),
    };
    let json = serde_json::to_string(&cape).unwrap();
    let deserialized: emerald_auth::Cape = serde_json::from_str(&json).unwrap();
    assert_eq!(deserialized.id, "cape-id");
    assert_eq!(deserialized.alias, "TestCape");
}
