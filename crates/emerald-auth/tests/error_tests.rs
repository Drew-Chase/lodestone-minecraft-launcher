use std::time::Duration;

use emerald_auth::AuthError;

#[test]
fn error_display_oauth() {
    let err = AuthError::OAuth {
        error: "access_denied".into(),
        description: "User cancelled".into(),
    };
    assert_eq!(err.to_string(), "oauth error: access_denied - User cancelled");
}

#[test]
fn error_display_state_mismatch() {
    let err = AuthError::StateMismatch {
        expected: "abc".into(),
        actual: "xyz".into(),
    };
    assert_eq!(
        err.to_string(),
        "csrf state mismatch: expected abc, got xyz"
    );
}

#[test]
fn error_display_xsts() {
    let err = AuthError::Xsts {
        xerr: 2148916233,
        message: "No Xbox account".into(),
    };
    assert_eq!(
        err.to_string(),
        "xsts authentication failed (xerr 2148916233): No Xbox account"
    );
}

#[test]
fn error_display_no_game_ownership() {
    let err = AuthError::NoGameOwnership;
    assert_eq!(err.to_string(), "account does not own Minecraft");
}

#[test]
fn error_display_timeout() {
    let err = AuthError::Timeout(Duration::from_secs(300));
    assert_eq!(err.to_string(), "authentication timed out after 300s");
}

#[test]
fn error_display_browser_open() {
    let err = AuthError::BrowserOpen("no display available".into());
    assert_eq!(
        err.to_string(),
        "failed to open browser: no display available"
    );
}

#[test]
fn error_display_missing_param() {
    let err = AuthError::MissingParam("code".into());
    assert_eq!(err.to_string(), "missing query parameter: code");
}

#[test]
fn error_display_xbox_live() {
    let err = AuthError::XboxLive("401: Unauthorized".into());
    assert_eq!(
        err.to_string(),
        "xbox live authentication failed: 401: Unauthorized"
    );
}

#[test]
fn error_display_minecraft() {
    let err = AuthError::Minecraft("invalid token".into());
    assert_eq!(
        err.to_string(),
        "minecraft authentication failed: invalid token"
    );
}

#[test]
fn io_error_converts_to_auth_error() {
    let io_err = std::io::Error::new(std::io::ErrorKind::AddrInUse, "port in use");
    let auth_err: AuthError = io_err.into();
    assert!(matches!(auth_err, AuthError::Io(_)));
    assert!(auth_err.to_string().contains("port in use"));
}

#[test]
fn url_parse_error_converts_to_auth_error() {
    let url_err = url::Url::parse("not a url").unwrap_err();
    let auth_err: AuthError = url_err.into();
    assert!(matches!(auth_err, AuthError::UrlParse(_)));
}

#[test]
fn json_error_converts_to_auth_error() {
    let json_err = serde_json::from_str::<serde_json::Value>("not json").unwrap_err();
    let auth_err: AuthError = json_err.into();
    assert!(matches!(auth_err, AuthError::Decode(_)));
}
