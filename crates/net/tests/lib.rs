use super::*;

#[test]
fn the_default_settings_follow_the_system_proxy_with_a_thirty_second_timeout() {
    assert_eq!(
        DEFAULT,
        Config {
            use_system_proxy: true,
            custom_proxy: None,
            timeout: Duration::from_secs(30),
            idle_per_host: 8,
        }
    );
}

#[test]
fn a_custom_proxy_the_client_cannot_use_is_skipped_and_the_client_still_builds() {
    set(Config {
        use_system_proxy: false,
        custom_proxy: Some("not a proxy url".to_owned()),
        timeout: Duration::from_secs(5),
        idle_per_host: 2,
    });
    let built = builder().build();
    set(DEFAULT);
    assert!(built.is_ok());
}

#[test]
fn recorded_bytes_add_up() {
    let before = bytes_received();
    record_bytes(1_000);
    record_bytes(24);
    assert!(bytes_received() >= before + 1_024);
}
