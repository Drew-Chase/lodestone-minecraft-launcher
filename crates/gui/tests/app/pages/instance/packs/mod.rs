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
        generation: 0,
        resource: vec![pack("New")],
        shader: Vec::new(),
    });
    assert_eq!(state.packs(ContentType::ResourcePack)[0].name, "New");
}

/// A record of the pack `file_name` in the folder `kind`, under `scope`, taken from `platform`.
fn record(kind: &str, scope: &str, platform: &str, file_name: &str) -> AddedContent {
    AddedContent {
        instance: "a".to_string(),
        scope: scope.to_string(),
        kind: kind.to_string(),
        platform: platform.to_string(),
        project_id: "p1".to_string(),
        version_id: "v1".to_string(),
        name: "Fresh".to_string(),
        file_name: file_name.to_string(),
    }
}

#[test]
fn a_pack_knows_the_platform_it_was_taken_from_by_its_kind_and_file_name() {
    let mut state = State::default();
    state.update(&Message::Sources {
        generation: 0,
        result: Ok(vec![
            record("resourcepacks", "", "modrinth", "Fresh.zip"),
            record("shaderpacks", "", "curseforge", "Shade.zip"),
        ]),
    });
    assert_eq!(state.source_of(ContentType::ResourcePack, "Fresh.zip"), Some(Source::Modrinth));
    assert_eq!(state.source_of(ContentType::Shader, "Shade.zip"), Some(Source::CurseForge));
    assert_eq!(state.source_of(ContentType::Shader, "Fresh.zip"), None);
}

#[test]
fn a_record_of_another_scope_or_an_unknown_platform_names_no_pack_source() {
    let mut state = State::default();
    state.update(&Message::Sources {
        generation: 0,
        result: Ok(vec![
            record("resourcepacks", "world-one", "modrinth", "Fresh.zip"),
            record("resourcepacks", "", "unknown", "Other.zip"),
        ]),
    });
    assert_eq!(state.source_of(ContentType::ResourcePack, "Fresh.zip"), None);
    assert_eq!(state.source_of(ContentType::ResourcePack, "Other.zip"), None);
}

#[test]
fn a_read_replaces_the_sources_it_had_and_a_failed_read_keeps_them() {
    let mut state = State::default();
    state.update(&Message::Sources {
        generation: 0,
        result: Ok(vec![record("resourcepacks", "", "modrinth", "Old.zip")]),
    });
    state.update(&Message::Sources {
        generation: 0,
        result: Err("the database is busy".to_string()),
    });
    assert_eq!(state.source_of(ContentType::ResourcePack, "Old.zip"), Some(Source::Modrinth));
    state.update(&Message::Sources {
        generation: 0,
        result: Ok(vec![record("resourcepacks", "", "ftb", "New.zip")]),
    });
    assert_eq!(state.source_of(ContentType::ResourcePack, "Old.zip"), None);
    assert_eq!(state.source_of(ContentType::ResourcePack, "New.zip"), Some(Source::Ftb));
}
