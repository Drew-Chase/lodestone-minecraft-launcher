use super::*;
use lodestone_platforms::ContentType;

/// A mod the folder holds, with the name and author given.
fn entry(file_name: &str, name: &str, author: Option<&str>, enabled: bool) -> ModFile {
    ModFile {
        file_name: file_name.to_string(),
        enabled,
        name: name.to_string(),
        author: author.map(str::to_string),
        version: None,
        icon: None,
    }
}

fn sample() -> State {
    let mut state = State::default();
    state.set_mods(vec![
        entry("sodium.jar", "Sodium", Some("jellysquid3"), true),
        entry("lithium.jar", "Lithium", Some("CaffeineMC"), true),
        entry("zoomies.jar.disabled", "Zoomies", None, false),
    ]);
    state
}

fn visible_names(state: &State) -> Vec<&str> {
    state.visible.iter().map(|&index| state.rows[index].file.name.as_str()).collect()
}

/// The record of the jar `file_name`, as the platform sources it from project `project_id`.
fn source(file_name: &str, project_id: &str) -> ModSource {
    ModSource {
        instance: "a".to_string(),
        platform: "modrinth".to_string(),
        project_id: project_id.to_string(),
        version_id: format!("{project_id}-1"),
        name: project_id.to_string(),
        file_name: file_name.to_string(),
        sha1: None,
    }
}

/// A release with the given id, which an update or a switch names.
fn release(id: &str) -> ContentVersion {
    ContentVersion {
        id: id.to_string(),
        project_id: Some("sodium".to_string()),
        name: None,
        version_number: id.to_string(),
        content_type: ContentType::Mod,
        minecraft_versions: Vec::new(),
        loaders: Vec::new(),
        downloads: 0,
        date: None,
        changelog_url: None,
        files: Vec::new(),
        changelog: None,
        channel: None,
        dependencies: Vec::new(),
    }
}

#[test]
fn an_empty_filter_shows_every_mod() {
    assert_eq!(
        visible_names(&sample()),
        ["Sodium", "Lithium", "Zoomies"],
        "kept in the order the folder was read"
    );
}

#[test]
fn text_matches_names_and_authors_ignoring_case() {
    let mut state = sample();
    let _ = state.update(Message::Query("CAFF".to_string()));
    assert_eq!(visible_names(&state), ["Lithium"]);
    let _ = state.update(Message::Query("sodium".to_string()));
    assert_eq!(visible_names(&state), ["Sodium"]);
}

#[test]
fn a_toggle_waits_for_the_root_to_rename_the_jar() {
    let mut state = sample();
    let before: Vec<bool> = state.rows.iter().map(|row| row.file.enabled).collect();
    let file_name = state.rows[1].file.file_name.clone();
    let _ = state.update(Message::Toggle { file_name, enabled: false });
    let after: Vec<bool> = state.rows.iter().map(|row| row.file.enabled).collect();
    assert_eq!(before, after, "the row switches only once the jar has been renamed");
}

#[test]
fn renamed_moves_the_row_to_the_new_name_and_switch() {
    let mut state = sample();
    let from = state.rows[1].file.file_name.clone();
    let to = format!("{from}.disabled");
    state.renamed(&from, &to, false);
    assert!(state.row(&from).is_none(), "the old name is gone");
    assert!(
        state.row(&to).is_some_and(|row| !row.file.enabled),
        "the new name is listed as switched off"
    );
}

#[test]
fn hovering_a_row_lifts_it_until_another_message_arrives() {
    let mut state = sample();
    let file_name = state.rows[0].file.file_name.clone();
    let _ = state.update(Message::Hover {
        file_name: file_name.clone(),
        hovered: true,
    });
    assert_eq!(state.hovered.as_deref(), Some(file_name.as_str()));
    let _ = state.update(Message::Scrolled(10.0));
    assert!(
        state.hovered.is_none(),
        "a scroll ends the hover, since the row under the pointer may have moved"
    );
}

#[test]
fn an_empty_folder_lists_nothing() {
    let mut state = State::default();
    state.set_mods(Vec::new());
    assert!(state.visible.is_empty());
}

#[test]
fn a_recorded_source_joins_its_mod_on_every_read_of_the_folder() {
    let mut state = sample();
    state.apply_sources(vec![source("sodium.jar", "sodium")]);
    assert_eq!(
        state
            .row("sodium.jar")
            .and_then(|row| row.source.as_ref())
            .map(|record| record.project_id.as_str()),
        Some("sodium")
    );

    state.set_mods(vec![entry("sodium.jar", "Sodium", None, true)]);
    assert!(
        state.row("sodium.jar").is_some_and(|row| row.source.is_some()),
        "a rescan keeps the source"
    );
}

#[test]
fn a_jar_that_is_gone_forgets_its_source() {
    let mut state = sample();
    state.apply_sources(vec![source("sodium.jar", "sodium")]);
    state.set_mods(vec![entry("lithium.jar", "Lithium", None, true)]);
    state.set_mods(vec![entry("sodium.jar", "Sodium", None, true)]);
    assert!(state.row("sodium.jar").is_some_and(|row| row.source.is_none()));
}

#[test]
fn a_recorded_source_is_applied_to_its_row_at_once() {
    let mut state = sample();
    state.apply_source(source("lithium.jar", "lithium"));
    assert!(state.row("lithium.jar").is_some_and(|row| row.source.is_some()));
    state.forget_source("lithium.jar");
    assert!(state.row("lithium.jar").is_some_and(|row| row.source.is_none()));
}

#[test]
fn identified_jars_gain_their_source_at_once() {
    let mut state = sample();
    state.apply_checks(vec![source("zoomies.jar.disabled", "zoomies")], HashMap::new(), &[]);
    assert!(state.row("zoomies.jar.disabled").is_some_and(|row| row.source.is_some()));
}

#[test]
fn a_check_gives_each_checked_mod_its_update_and_clears_the_rest() {
    let mut state = sample();
    let found = HashMap::from([("sodium.jar".to_string(), release("sodium-2"))]);
    state.apply_checks(Vec::new(), found, &["sodium.jar".to_string(), "lithium.jar".to_string()]);
    assert!(state.row("sodium.jar").is_some_and(|row| row.update.is_some()));
    assert!(state.row("lithium.jar").is_some_and(|row| row.update.is_none()));

    state.apply_checks(Vec::new(), HashMap::new(), &["sodium.jar".to_string()]);
    assert!(
        state.row("sodium.jar").is_some_and(|row| row.update.is_none()),
        "a check that finds nothing clears the update"
    );
}

#[test]
fn an_update_outlives_a_rescan_until_the_check_clears_it() {
    let mut state = sample();
    state.apply_checks(
        Vec::new(),
        HashMap::from([("sodium.jar".to_string(), release("sodium-2"))]),
        &["sodium.jar".to_string()],
    );
    state.set_mods(vec![entry("sodium.jar", "Sodium", None, true)]);
    assert!(state.row("sodium.jar").is_some_and(|row| row.update.is_some()));
}

#[test]
fn the_updates_chip_narrows_the_list_to_the_mods_with_a_newer_release() {
    let mut state = sample();
    let found = HashMap::from([("lithium.jar".to_string(), release("lithium-2"))]);
    state.apply_checks(Vec::new(), found, &["sodium.jar".to_string(), "lithium.jar".to_string()]);
    assert_eq!(state.update_count(), 1);

    let _ = state.update(Message::Updates(true));
    assert_eq!(visible_names(&state), ["Lithium"]);

    let _ = state.update(Message::Query("sod".to_string()));
    assert!(state.visible.is_empty(), "the chip and the text both have to pass");

    let _ = state.update(Message::Updates(false));
    assert_eq!(visible_names(&state), ["Sodium"]);
}

#[test]
fn scrolling_is_remembered_and_a_new_filter_goes_back_to_the_top() {
    let mut state = sample();
    let _ = state.update(Message::Scrolled(300.0));
    assert_eq!(state.scroll_y, 300.0);
    let _ = state.update(Message::Query("lith".to_string()));
    assert_eq!(state.scroll_y, 0.0);
}

#[test]
fn a_mod_being_acted_on_is_marked_until_it_lands() {
    let mut state = sample();
    state.set_busy("sodium.jar", true);
    assert!(state.busy.contains("sodium.jar"));
    state.set_busy("sodium.jar", false);
    assert!(!state.busy.contains("sodium.jar"));
}
