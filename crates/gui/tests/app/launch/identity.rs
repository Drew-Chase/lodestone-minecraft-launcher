use super::*;

#[test]
fn the_offline_uuid_matches_the_vanilla_server() {
    // MD5 of "OfflinePlayer:Notch", with the version-3 and IETF variant bits set.
    assert_eq!(offline_uuid("Notch"), "b50ad385829d3141a2167e7d7539ba7f");
}

#[test]
fn the_offline_uuid_is_version_three_and_dashless() {
    let uuid = offline_uuid("Steve");
    assert_eq!(uuid.len(), 32);
    assert!(!uuid.contains('-'));
    assert_eq!(&uuid[12..13], "3");
    assert!(matches!(&uuid[16..17], "8" | "9" | "a" | "b"));
}

#[test]
fn an_offline_player_plays_as_a_legacy_user_with_token_zero() {
    let identity = Identity::offline("Steve");
    assert_eq!(identity.name, "Steve");
    assert_eq!(identity.uuid, offline_uuid("Steve"));
    assert_eq!(identity.access_token, "0");
    assert_eq!(identity.user_type, "legacy");
    assert_eq!(identity.xuid, "0");
}

#[test]
fn a_microsoft_player_keeps_the_token_and_drops_the_uuid_dashes() {
    let identity = Identity::microsoft("Alex", "b4ccccad-1111-2222-3333-444455556666", "eyJ.token");
    assert_eq!(identity.uuid, "b4ccccad111122223333444455556666");
    assert_eq!(identity.access_token, "eyJ.token");
    assert_eq!(identity.user_type, "msa");
    assert_eq!(identity.name, "Alex");
}
