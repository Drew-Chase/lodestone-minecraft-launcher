use emerald_auth::microsoft;

#[test]
fn build_auth_url_contains_required_params() {
    let url = microsoft::build_auth_url("my-client-id", "http://localhost:8080/callback", "random-state");

    assert!(url.starts_with("https://login.microsoftonline.com/consumers/oauth2/v2.0/authorize?"));
    assert!(url.contains("client_id=my-client-id"));
    assert!(url.contains("response_type=code"));
    assert!(url.contains("redirect_uri=http%3A%2F%2Flocalhost%3A8080%2Fcallback"));
    assert!(url.contains("scope=XboxLive.signin+offline_access"));
    assert!(url.contains("state=random-state"));
    assert!(url.contains("prompt=select_account"));
}

#[test]
fn build_auth_url_encodes_special_chars_in_redirect() {
    let url = microsoft::build_auth_url("id", "http://localhost:1234/my callback", "s");
    // Space in redirect_uri should be percent-encoded.
    assert!(url.contains("redirect_uri=http%3A%2F%2Flocalhost%3A1234%2Fmy+callback")
        || url.contains("redirect_uri=http%3A%2F%2Flocalhost%3A1234%2Fmy%20callback"));
}

#[test]
fn build_auth_url_preserves_client_id_and_state() {
    let url = microsoft::build_auth_url("abc-def-123", "http://localhost:9999/cb", "state-token-456");
    assert!(url.contains("client_id=abc-def-123"));
    assert!(url.contains("state=state-token-456"));
}
