use super::*;

#[test]
fn a_path_inside_the_instance_is_named_with_forward_slashes() {
    let root = Path::new("/instances/aether");
    assert_eq!(
        relative(root, &root.join("config").join("sodium.json")),
        Some("config/sodium.json".to_string())
    );
    assert_eq!(relative(root, root), Some(String::new()), "the instance folder is the empty path");
    assert_eq!(relative(root, Path::new("/elsewhere/a.txt")), None);
}

#[test]
fn an_entry_is_named_by_its_file_name() {
    assert_eq!(name_of(Path::new("/instances/aether/config/sodium.json")), "sodium.json");
    assert_eq!(name_of(Path::new("config")), "config");
}
