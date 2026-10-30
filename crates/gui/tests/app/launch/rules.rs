use super::*;

fn rule(json: &str) -> Rule {
    serde_json::from_str(json).expect("the rule parses")
}

const WINDOWS: Platform = Platform {
    os: "windows",
    arch: "x86_64",
};

#[test]
fn a_missing_rule_list_allows() {
    assert!(allowed(None, WINDOWS));
}

#[test]
fn a_present_rule_list_allows_nothing_when_no_rule_applies() {
    assert!(!allowed(Some(&[]), WINDOWS));
    let mac_only = [rule(r#"{"action":"allow","os":{"name":"osx"}}"#)];
    assert!(!allowed(Some(&mac_only), WINDOWS));
}

#[test]
fn an_os_rule_applies_on_its_own_system_only() {
    let windows_only = [rule(r#"{"action":"allow","os":{"name":"windows"}}"#)];
    assert!(allowed(Some(&windows_only), WINDOWS));
    assert!(!allowed(Some(&windows_only), Platform { os: "linux", arch: "x86_64" }));
}

#[test]
fn the_last_applicable_rule_decides() {
    let rules = [rule(r#"{"action":"allow"}"#), rule(r#"{"action":"disallow","os":{"name":"osx"}}"#)];
    assert!(allowed(Some(&rules), WINDOWS));
    assert!(!allowed(Some(&rules), Platform { os: "osx", arch: "arm64" }));
    let disallowed_last = [
        rule(r#"{"action":"allow","os":{"name":"windows"}}"#),
        rule(r#"{"action":"disallow","os":{"name":"windows"}}"#),
    ];
    assert!(!allowed(Some(&disallowed_last), WINDOWS));
}

#[test]
fn an_arch_rule_matches_its_architecture_only() {
    let thirty_two_bit = [rule(r#"{"action":"allow","os":{"name":"windows","arch":"x86"}}"#)];
    assert!(!allowed(Some(&thirty_two_bit), WINDOWS));
    assert!(allowed(Some(&thirty_two_bit), Platform { os: "windows", arch: "x86" }));
}

#[test]
fn a_feature_rule_needs_its_feature_off() {
    let needs_resolution = [rule(r#"{"action":"allow","features":{"has_custom_resolution":true}}"#)];
    assert!(!allowed(Some(&needs_resolution), WINDOWS));
    let needs_no_demo = [rule(r#"{"action":"allow","features":{"is_demo_user":false}}"#)];
    assert!(allowed(Some(&needs_no_demo), WINDOWS));
}

#[test]
fn the_platform_names_match_mojang() {
    assert_eq!(
        Platform {
            os: "windows",
            arch: "x86_64"
        }
        .classpath_separator(),
        ';'
    );
    assert_eq!(Platform { os: "linux", arch: "x86_64" }.classpath_separator(), ':');
    if cfg!(windows) {
        assert_eq!(Platform::current().os, "windows");
    }
}
