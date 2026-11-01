use super::*;

#[test]
fn a_string_icon_is_its_path() {
    assert_eq!(
        largest(&serde_json::json!("assets/testmod/icon.png")).as_deref(),
        Some("assets/testmod/icon.png")
    );
}

#[test]
fn a_map_of_sizes_names_its_largest_picture() {
    let icon = serde_json::json!({ "16": "assets/a/16.png", "128": "assets/a/128.png", "32": "assets/a/32.png" });
    assert_eq!(largest(&icon).as_deref(), Some("assets/a/128.png"));
}

#[test]
fn a_size_that_is_not_a_number_is_skipped() {
    let icon = serde_json::json!({ "big": "assets/a/big.png", "64": "assets/a/64.png" });
    assert_eq!(largest(&icon).as_deref(), Some("assets/a/64.png"));
}

#[test]
fn no_icon_is_none() {
    assert_eq!(largest(&serde_json::json!(null)), None);
    assert_eq!(largest(&serde_json::json!({})), None);
}
