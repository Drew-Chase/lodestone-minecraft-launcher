use std::time::Duration;

use emerald_auth::MicrosoftAuth;

#[test]
fn builder_defaults() {
    let auth = MicrosoftAuth::new("test-client-id");
    // Just verify it constructs without panic.
    drop(auth);
}

#[test]
fn builder_with_timeout() {
    let auth = MicrosoftAuth::new("test-client-id")
        .with_timeout(Duration::from_secs(60));
    drop(auth);
}

#[test]
fn builder_with_port() {
    let auth = MicrosoftAuth::new("test-client-id")
        .with_port(25585);
    drop(auth);
}

#[test]
fn builder_with_custom_client() {
    let client = reqwest::Client::builder()
        .user_agent("test-agent/1.0")
        .build()
        .unwrap();
    let auth = MicrosoftAuth::new("test-client-id")
        .with_http_client(client);
    drop(auth);
}

#[test]
fn builder_chaining() {
    let auth = MicrosoftAuth::new("test-client-id")
        .with_timeout(Duration::from_secs(120))
        .with_port(8080)
        .with_http_client(reqwest::Client::new());
    drop(auth);
}
