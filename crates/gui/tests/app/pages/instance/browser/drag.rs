use super::*;

fn paths(names: &[&str]) -> Vec<String> {
    names.iter().map(|name| (*name).to_string()).collect()
}

#[test]
fn a_press_that_is_released_in_place_is_a_click() {
    let mut drag = Drag::press("a.txt".to_string());
    assert!(drag.is_armed());
    drag.moved(Point::new(10.0, 10.0), &[]);
    drag.moved(Point::new(12.0, 11.0), &[]);
    assert_eq!(drag.dragged(), None, "a wobble is not a drag");
    assert_eq!(drag.release(), Released::Click("a.txt".to_string()));
    assert!(!drag.is_armed());
}

#[test]
fn a_press_becomes_a_drag_once_the_pointer_has_moved_far_enough() {
    let mut drag = Drag::press("a.txt".to_string());
    drag.moved(Point::new(10.0, 10.0), &[]);
    drag.moved(Point::new(10.0, 10.0 + THRESHOLD), &[]);
    assert_eq!(drag.dragged(), Some(paths(&["a.txt"]).as_slice()));
    drag.moved(Point::new(40.0, 80.0), &[]);
    assert_eq!(drag.pointer(), Some(Point::new(40.0, 80.0)));
    assert_eq!(drag.release(), Released::Drop(paths(&["a.txt"])));
}

#[test]
fn dragging_a_selected_row_takes_the_whole_selection() {
    let selection = paths(&["a.txt", "b.txt"]);
    let mut drag = Drag::press("b.txt".to_string());
    drag.moved(Point::ORIGIN, &selection);
    drag.moved(Point::new(20.0, 0.0), &selection);
    assert_eq!(drag.dragged(), Some(selection.as_slice()));
}

#[test]
fn dragging_a_row_outside_the_selection_takes_that_row_alone() {
    let selection = paths(&["a.txt", "b.txt"]);
    let mut drag = Drag::press("c.txt".to_string());
    drag.moved(Point::ORIGIN, &selection);
    drag.moved(Point::new(20.0, 0.0), &selection);
    assert_eq!(drag.dragged(), Some(paths(&["c.txt"]).as_slice()));
}

#[test]
fn a_release_with_nothing_pressed_is_nothing() {
    let mut drag = Drag::default();
    drag.moved(Point::new(50.0, 50.0), &[]);
    assert_eq!(drag.release(), Released::Nothing);
}

#[test]
fn a_folder_takes_rows_from_elsewhere_but_not_itself_or_what_it_already_holds() {
    let dragged = paths(&["config/a.toml", "config/fabric"]);
    assert!(accepts("mods", &dragged));
    assert!(accepts("", &dragged), "the instance folder takes rows from a folder below it");
    assert!(!accepts("config", &dragged), "the rows are already in that folder");
    assert!(!accepts("config/fabric", &dragged), "a folder does not go into itself");
    assert!(!accepts("config/fabric/inner", &dragged), "or into a folder inside it");
    assert!(
        accepts("config/fabric-api", &dragged),
        "a name that only starts the same is another folder"
    );
}

#[test]
fn the_parent_of_a_top_level_entry_is_the_instance_folder() {
    assert_eq!(parent("options.txt"), "");
    assert_eq!(parent("config/fabric/a.json"), "config/fabric");
}
