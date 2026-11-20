use super::*;

fn entry(path: &str, is_folder: bool) -> Row {
    Row {
        name: path.rsplit('/').next().unwrap_or(path).to_string(),
        path: path.to_string(),
        is_folder,
        is_link: false,
        size: 0,
        modified: None,
    }
}

/// A tab on the instance folder `/inst`, with its top folder read as two folders and three files.
fn listed(read_only: bool) -> State {
    let mut state = State::new(PathBuf::from("/inst"), read_only);
    let _ = state.open();
    deliver(
        &mut state,
        vec![
            entry("b.txt", false),
            entry("mods", true),
            entry("a.txt", false),
            entry("config", true),
            entry("c.txt", false),
        ],
    );
    state
}

/// Lands the read the tab is waiting for, with `rows`.
fn deliver(state: &mut State, rows: Vec<Row>) {
    let message = Message::Listed {
        root: state.root.clone(),
        request: state.request,
        folder: state.folder.clone(),
        result: Ok(rows),
    };
    let _ = state.update(message);
}

fn names(state: &State) -> Vec<&str> {
    state.order.iter().map(|&index| state.rows[index].name.as_str()).collect()
}

/// Presses and releases the row at `path`, as a click does.
fn click(state: &mut State, path: &str) {
    let _ = state.update(Message::Press(path.to_string()));
    let _ = state.update(Message::Released);
}

#[test]
fn a_folder_is_listed_with_its_folders_first() {
    let state = listed(false);
    assert!(state.listed);
    assert_eq!(names(&state), ["config", "mods", "a.txt", "b.txt", "c.txt"]);
}

#[test]
fn a_read_that_is_not_the_newest_is_dropped() {
    let mut state = listed(false);
    let _ = state.refresh();
    let stale = Message::Listed {
        root: state.root.clone(),
        request: state.request - 1,
        folder: String::new(),
        result: Ok(Vec::new()),
    };
    let _ = state.update(stale);
    assert_eq!(state.rows.len(), 5, "the older read does not replace the rows");

    let other = Message::Listed {
        root: PathBuf::from("/another"),
        request: state.request,
        folder: String::new(),
        result: Ok(Vec::new()),
    };
    let _ = state.update(other);
    assert_eq!(state.rows.len(), 5, "nor does a read of another instance");
}

#[test]
fn opening_a_folder_shows_it_and_up_goes_back() {
    let mut state = listed(false);
    click(&mut state, "a.txt");
    let _ = state.update(Message::Open("config".to_string()));
    assert_eq!(state.folder(), "config");
    assert!(!state.listed, "the folder is read before its rows show");
    assert!(state.selection.is_empty());
    deliver(&mut state, vec![entry("config/fabric", true)]);
    let _ = state.update(Message::Open("config/fabric".to_string()));
    let _ = state.update(Message::Up);
    assert_eq!(state.folder(), "config");
    let _ = state.update(Message::Up);
    assert_eq!(state.folder(), "");
    let _ = state.update(Message::Up);
    assert_eq!(state.folder(), "", "the instance folder has nothing above it");
}

#[test]
fn a_folder_outside_the_instance_is_not_opened() {
    let mut state = listed(false);
    let _ = state.update(Message::Open("../elsewhere".to_string()));
    assert_eq!(state.folder(), "");
}

#[test]
fn two_presses_close_together_open_a_folder() {
    let mut state = listed(false);
    click(&mut state, "mods");
    assert_eq!(state.folder(), "", "one press only selects");
    assert_eq!(state.selection.paths(), ["mods"]);
    click(&mut state, "mods");
    assert_eq!(state.folder(), "mods");
}

#[test]
fn presses_on_two_rows_are_not_a_double_click() {
    let mut state = listed(false);
    click(&mut state, "config");
    click(&mut state, "mods");
    assert_eq!(state.folder(), "");
    assert_eq!(state.selection.paths(), ["mods"]);
}

#[test]
fn the_command_and_shift_keys_build_a_selection() {
    let mut state = listed(false);
    click(&mut state, "a.txt");
    let _ = state.update(Message::Held(Held { command: true, shift: false }));
    click(&mut state, "c.txt");
    assert_eq!(state.selection.paths(), ["a.txt", "c.txt"]);
    let _ = state.update(Message::Held(Held { command: false, shift: true }));
    click(&mut state, "mods");
    assert_eq!(
        state.selection.paths(),
        ["a.txt", "b.txt", "c.txt", "mods"],
        "the range runs in the order on screen"
    );
}

#[test]
fn a_press_on_a_selected_row_keeps_the_selection_until_the_release() {
    let mut state = listed(false);
    let _ = state.update(Message::Key(Key::SelectAll));
    let _ = state.update(Message::Press("a.txt".to_string()));
    assert_eq!(state.selection.len(), 5, "the selection stays, so all of it can be dragged");
    let _ = state.update(Message::Released);
    assert_eq!(state.selection.paths(), ["a.txt"], "a press that was not a drag selects its row alone");
}

#[test]
fn dragging_past_the_threshold_drags_the_selection_and_escape_cancels() {
    let mut state = listed(false);
    let _ = state.update(Message::Key(Key::SelectAll));
    let _ = state.update(Message::Press("a.txt".to_string()));
    let _ = state.update(Message::Moved(Point::new(10.0, 10.0)));
    let _ = state.update(Message::Moved(Point::new(10.0, 40.0)));
    assert_eq!(state.drag.dragged().map(<[String]>::len), Some(5));
    let _ = state.update(Message::Key(Key::Escape { taken: false }));
    assert!(!state.drag.is_armed());
    assert_eq!(state.selection.len(), 5, "cancelling a drag leaves the selection");
}

#[test]
fn a_drop_lands_on_a_folder_that_can_take_the_rows() {
    let mut state = listed(false);
    let dragged = vec!["a.txt".to_string()];
    assert_eq!(state.drop_target(&dragged), None, "nothing is under the pointer");
    let _ = state.update(Message::Hover {
        path: "mods".to_string(),
        hovered: true,
    });
    assert_eq!(state.drop_target(&dragged), Some("mods".to_string()));
    assert_eq!(state.drop_target(&["mods".to_string()]), None, "a folder does not take itself");
    let _ = state.update(Message::Hover {
        path: "b.txt".to_string(),
        hovered: true,
    });
    assert_eq!(state.drop_target(&dragged), None, "a file takes nothing");
}

#[test]
fn a_breadcrumb_takes_a_drop_from_a_folder_below_it() {
    let mut state = listed(false);
    let _ = state.update(Message::Open("config".to_string()));
    deliver(&mut state, vec![entry("config/a.toml", false)]);
    let dragged = vec!["config/a.toml".to_string()];
    let _ = state.update(Message::CrumbHover {
        folder: "config".to_string(),
        hovered: true,
    });
    assert_eq!(state.drop_target(&dragged), None, "the row is already in that folder");
    let _ = state.update(Message::CrumbHover {
        folder: String::new(),
        hovered: true,
    });
    assert_eq!(state.drop_target(&dragged), Some(String::new()));
}

#[test]
fn the_arrow_keys_move_the_selection_through_the_rows_on_screen() {
    let mut state = listed(false);
    let _ = state.update(Message::Key(Key::Step { delta: 1, extend: false }));
    assert_eq!(state.selection.paths(), ["config"]);
    let _ = state.update(Message::Key(Key::Step { delta: 2, extend: true }));
    assert_eq!(state.selection.paths(), ["a.txt", "config", "mods"]);
    let _ = state.update(Message::Key(Key::Open));
    assert_eq!(state.folder(), "", "Enter opens only a single selection");
}

#[test]
fn enter_opens_the_selected_folder_and_backspace_goes_up() {
    let mut state = listed(false);
    click(&mut state, "config");
    let _ = state.update(Message::Key(Key::Open));
    assert_eq!(state.folder(), "config");
    let _ = state.update(Message::Key(Key::Up));
    assert_eq!(state.folder(), "");
}

#[test]
fn renaming_shows_the_field_with_the_name_and_escape_ends_it() {
    let mut state = listed(false);
    click(&mut state, "a.txt");
    let _ = state.update(Message::Key(Key::Rename));
    assert_eq!(state.rename.as_ref().map(|rename| rename.text.as_str()), Some("a.txt"));
    let _ = state.update(Message::RenameInput("notes.txt".to_string()));
    // The field takes Escape first, to drop its focus. The rename still ends, and the selection stays.
    let _ = state.update(Message::Key(Key::Escape { taken: true }));
    assert!(state.rename.is_none());
    assert_eq!(state.selection.paths(), ["a.txt"]);
}

#[test]
fn escape_backs_out_of_one_thing_at_a_time() {
    let mut state = listed(false);
    click(&mut state, "a.txt");
    state.set_clip(Some(Clip {
        paths: vec![PathBuf::from("/inst/a.txt")],
        cut: true,
    }));
    assert!(state.is_cut("a.txt"));
    let _ = state.update(Message::Key(Key::Escape { taken: false }));
    assert!(state.clip().is_none(), "a cut that waits for its paste is dropped first");
    assert_eq!(state.selection.paths(), ["a.txt"]);
    let _ = state.update(Message::Key(Key::Escape { taken: false }));
    assert!(state.selection.is_empty());
}

#[test]
fn copied_files_are_not_drawn_as_cut() {
    let mut state = listed(false);
    state.set_clip(Some(Clip {
        paths: vec![PathBuf::from("/inst/a.txt")],
        cut: false,
    }));
    assert!(!state.is_cut("a.txt"));
}

#[test]
fn a_read_only_tab_selects_and_opens_but_starts_no_drag_and_no_rename() {
    let mut state = listed(true);
    let _ = state.update(Message::Press("a.txt".to_string()));
    assert!(!state.drag.is_armed());
    assert_eq!(state.selection.paths(), ["a.txt"]);
    let _ = state.update(Message::Key(Key::Rename));
    assert!(state.rename.is_none());
    let _ = state.update(Message::Menu {
        path: Some("a.txt".to_string()),
        action: Some(Action::Rename),
    });
    assert!(state.rename.is_none());
    let _ = state.update(Message::FileHover(true));
    assert!(!state.file_hover, "a locked instance shows no drop cover");
    let _ = state.update(Message::Dropped(PathBuf::from("/home/me/pack.zip")));
    assert!(state.drops.is_empty());
    click(&mut state, "mods");
    click(&mut state, "mods");
    assert_eq!(state.folder(), "mods", "folders still open");
}

#[test]
fn files_dropped_together_are_gathered_into_one_job() {
    let mut state = listed(false);
    let _ = state.update(Message::FileHover(true));
    assert!(state.file_hover);
    let _ = state.update(Message::Dropped(PathBuf::from("/home/me/a.zip")));
    let _ = state.update(Message::Dropped(PathBuf::from("/home/me/b.zip")));
    assert!(!state.file_hover, "the cover goes once the files land");
    assert_eq!(state.drops.len(), 2);
    let _ = state.update(Message::DropsGathered);
    assert!(state.drops.is_empty());
}

#[test]
fn a_reread_keeps_the_rows_still_there_selected_and_selects_what_was_just_written() {
    let mut state = listed(false);
    let _ = state.update(Message::Key(Key::SelectAll));
    let _ = state.refresh();
    deliver(&mut state, vec![entry("a.txt", false), entry("mods", true)]);
    assert_eq!(state.selection.paths(), ["a.txt", "mods"]);

    state.select_when_listed(vec!["pasted.txt".to_string(), "elsewhere/x.txt".to_string()]);
    let _ = state.refresh();
    deliver(&mut state, vec![entry("a.txt", false), entry("pasted.txt", false)]);
    assert_eq!(state.selection.paths(), ["pasted.txt"], "only what this folder holds is selected");
}

#[test]
fn a_new_entry_is_renamed_as_soon_as_it_is_listed() {
    let mut state = listed(false);
    state.rename_when_listed("New folder".to_string());
    let _ = state.refresh();
    deliver(&mut state, vec![entry("New folder", true), entry("a.txt", false)]);
    assert_eq!(state.rename.as_ref().map(|rename| rename.path.as_str()), Some("New folder"));
    assert_eq!(state.selection.paths(), ["New folder"]);
}

#[test]
fn a_change_is_read_only_for_the_folder_on_screen() {
    let mut state = listed(false);
    let before = state.request;
    let _ = state.update(Message::Changed(PathBuf::from("/inst/config")));
    assert_eq!(state.request, before, "another folder changed, so nothing is read");
    let _ = state.update(Message::Changed(PathBuf::from("/inst")));
    assert_eq!(state.request, before + 1);
}

#[test]
fn sorting_by_a_column_again_flips_its_direction() {
    let mut state = listed(false);
    let _ = state.update(Message::Sort(Column::Name));
    assert_eq!(
        names(&state),
        ["mods", "config", "c.txt", "b.txt", "a.txt"],
        "folders still lead a descending sort"
    );
}

#[test]
fn locking_the_instance_drops_a_rename_and_a_drag_under_way() {
    let mut state = listed(false);
    click(&mut state, "a.txt");
    let _ = state.update(Message::Key(Key::Rename));
    state.set_read_only(true);
    assert!(state.rename.is_none());
    assert!(state.read_only());
}

#[test]
fn opening_a_folder_remembers_the_one_left_and_back_returns_to_it() {
    let mut state = listed(false);
    let _ = state.update(Message::Open("mods".to_string()));
    assert_eq!(state.folder(), "mods");
    assert!(state.can_step_back());
    assert!(!state.can_step_forward());

    let _ = state.step_back();
    assert_eq!(state.folder(), "");
    assert!(!state.can_step_back());
    assert!(state.can_step_forward());

    let _ = state.step_forward();
    assert_eq!(state.folder(), "mods");
}

#[test]
fn a_new_step_after_going_back_drops_the_steps_forward() {
    let mut state = listed(false);
    let _ = state.update(Message::Open("mods".to_string()));
    let _ = state.step_back();
    let _ = state.update(Message::Open("config".to_string()));
    assert_eq!(state.folder(), "config");
    assert!(
        !state.can_step_forward(),
        "a folder opened after going back leaves nothing to go forward to"
    );
}

#[test]
fn up_is_a_step_too_and_opening_the_folder_already_on_screen_is_not() {
    let mut state = listed(false);
    let _ = state.update(Message::Open("mods".to_string()));
    let _ = state.update(Message::Open("mods".to_string()));
    let _ = state.update(Message::Up);
    assert_eq!(state.folder(), "");
    let _ = state.step_back();
    assert_eq!(state.folder(), "mods", "Up is remembered, and the repeated open was not");
}
