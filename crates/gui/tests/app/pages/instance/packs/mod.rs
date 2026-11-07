use super::*;

/// A zip pack of the given name, as the folder scan reads it.
fn pack(name: &str) -> PackFile {
    PackFile {
        file_name: format!("{name}.zip"),
        name: name.to_string(),
        size: Some(1),
        is_dir: false,
    }
}

#[test]
fn each_list_shows_the_packs_its_folder_holds() {
    let mut state = State::default();
    state.set_packs(vec![pack("Fresh")], vec![pack("Complementary")]);
    assert_eq!(state.packs(ContentType::ResourcePack)[0].name, "Fresh");
    assert_eq!(state.packs(ContentType::Shader)[0].name, "Complementary");
}

#[test]
fn the_filter_is_kept_by_the_tab() {
    let mut state = State::default();
    state.update(&Message::Query("fresh".to_string()));
    assert_eq!(state.query, "fresh");
}

#[test]
fn a_shared_folder_read_opens_the_dialog_and_closing_it_shuts_the_dialog() {
    let mut state = State::default();
    state.update(&Message::SharedLoaded {
        kind: ContentType::ResourcePack,
        entries: Vec::new(),
    });
    assert!(state.dialog().is_some());
    state.update(&Message::CloseShared);
    assert!(state.dialog().is_none());
}

#[test]
fn a_rescan_replaces_the_packs_the_lists_show() {
    let mut state = State::default();
    state.set_packs(vec![pack("Old")], Vec::new());
    state.update(&Message::Rescanned {
        resource: vec![pack("New")],
        shader: Vec::new(),
    });
    assert_eq!(state.packs(ContentType::ResourcePack)[0].name, "New");
}
