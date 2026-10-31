use lodestone_minecraft_authentication::{Profile, Session};

use super::*;

/// The moment every check is made, in seconds since the Unix epoch.
const NOW: u64 = 1_700_000_000;

#[test]
fn quick_play_needs_a_version_from_1_20_on() {
    assert!(supports_quick_play("1.20"));
    assert!(supports_quick_play("1.20.1"));
    assert!(supports_quick_play("1.21.4"));
    assert!(!supports_quick_play("1.19.4"));
    assert!(!supports_quick_play("24w14a"), "a snapshot starts the instance alone");
    assert!(!supports_quick_play("1"), "a version without a minor part starts the instance alone");
}

/// A Microsoft account whose session expires `in_seconds` after [`NOW`], holding `refresh_token`.
fn microsoft(in_seconds: u64, refresh_token: Option<&str>) -> Account {
    Account::Microsoft {
        session: Session {
            minecraft_token: "minecraft-token".to_string(),
            expires_at: NOW + in_seconds,
            refresh_token: refresh_token.map(str::to_string),
            profile: Profile {
                id: "0123456789abcdef".to_string(),
                name: "Drew".to_string(),
                skin_url: None,
            },
        },
    }
}

#[test]
fn an_offline_account_never_needs_a_refresh() {
    let offline = Account::Offline {
        username: "Drew".to_string(),
    };
    assert_eq!(session_check(&offline, NOW), SessionCheck::Fresh);
}

#[test]
fn a_session_far_from_expiry_is_fresh() {
    assert_eq!(session_check(&microsoft(REFRESH_WITHIN + 1, Some("r1")), NOW), SessionCheck::Fresh);
}

#[test]
fn a_session_close_to_expiry_is_refreshed_with_its_refresh_token() {
    assert_eq!(
        session_check(&microsoft(REFRESH_WITHIN, Some("r1")), NOW),
        SessionCheck::Refresh("r1".to_string())
    );
    assert_eq!(session_check(&microsoft(0, Some("r1")), NOW), SessionCheck::Refresh("r1".to_string()));
}

#[test]
fn a_session_close_to_expiry_without_a_refresh_token_needs_a_sign_in() {
    assert_eq!(session_check(&microsoft(10, None), NOW), SessionCheck::SignInAgain);
}
