use super::*;

#[test]
fn ordinary_names_keep_their_spaces() {
    assert_eq!(clean_directory_name("Dragon Survival v2"), "Dragon Survival v2");
}

#[test]
fn rejected_characters_are_dropped() {
    assert_eq!(clean_directory_name("A/B: test?"), "AB test");
    assert_eq!(clean_directory_name(r#"a<b>c"d|e*f\g"#), "abcdefg");
}

#[test]
fn control_characters_are_dropped() {
    assert_eq!(clean_directory_name("Tab\tName\n"), "TabName");
}

#[test]
fn leading_and_trailing_dots_and_spaces_are_trimmed() {
    assert_eq!(clean_directory_name("  ..Pack..  "), "Pack");
    assert_eq!(clean_directory_name("Pack. ."), "Pack");
}

#[test]
fn reserved_device_names_get_an_underscore() {
    assert_eq!(clean_directory_name("con"), "_con");
    assert_eq!(clean_directory_name("NUL.txt"), "_NUL.txt");
    assert_eq!(clean_directory_name("COM1"), "_COM1");
    assert_eq!(clean_directory_name("lpt9"), "_lpt9");
    assert_eq!(clean_directory_name("Comet"), "Comet", "only whole device names are reserved");
}

#[test]
fn names_with_nothing_printable_fall_back_to_instance() {
    assert_eq!(clean_directory_name(""), "Instance");
    assert_eq!(clean_directory_name("???"), "Instance");
    assert_eq!(clean_directory_name(" .. "), "Instance");
}

#[test]
fn long_names_are_capped() {
    let name = "x".repeat(200);
    assert_eq!(clean_directory_name(&name).chars().count(), MAX_CHARS);
}

#[test]
fn unique_names_count_up_past_existing_folders() {
    let dir = std::env::temp_dir().join(format!("lodestone-naming-{}", std::process::id()));
    std::fs::create_dir_all(dir.join("Pack")).expect("the temp folder creates");
    std::fs::create_dir_all(dir.join("Pack (2)")).expect("the second folder creates");

    assert_eq!(unique_directory_name(&dir, "Pack"), "Pack (3)");
    assert_eq!(unique_directory_name(&dir, "Other"), "Other", "a free name is used as it is");

    let _ = std::fs::remove_dir_all(&dir);
}
