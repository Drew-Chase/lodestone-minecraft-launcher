use super::*;

fn items() -> Vec<MenuItem> {
    vec![
        MenuItem::Action {
            label: String::from("Open"),
            icon: None,
            danger: false,
        },
        MenuItem::Separator,
        MenuItem::Action {
            label: String::from("Delete"),
            icon: None,
            danger: true,
        },
    ]
}

#[test]
fn heights_follow_the_spec() {
    let items = items();
    assert_eq!(items[0].height(), ROW_HEIGHT);
    assert_eq!(items[1].height(), SEPARATOR_HEIGHT);
}

#[test]
fn open_close_lifecycle() {
    let mut state = MenuState::new();
    assert!(state.is_closed());

    state.open();
    assert!(state.is_open());
    assert_eq!(state.open.target(), &1.0);

    state.close();
    assert_eq!(state.phase, Phase::Closing);
    assert_eq!(state.open.target(), &0.0);

    // Closing is idempotent; closed menus stay closed.
    state.close();
    assert_eq!(state.phase, Phase::Closing);
    state.phase = Phase::Closed;
    state.close();
    assert!(state.is_closed());
}

#[test]
fn selectability() {
    let items = items();
    assert!(items[0].is_selectable());
    assert!(!items[1].is_selectable());
}

#[test]
fn an_empty_query_lists_every_item() {
    assert_eq!(listed(&items(), ""), vec![0, 1, 2]);
}

#[test]
fn a_query_lists_only_matching_actions_ignoring_case() {
    assert_eq!(listed(&items(), "DEL"), vec![2]);
    assert_eq!(listed(&items(), "open"), vec![0]);
    assert!(listed(&items(), "zzz").is_empty(), "a query can match nothing");
}

#[test]
fn a_query_hides_the_separators() {
    let items = items();
    assert!(listed(&items, "e").iter().all(|&index| items[index].is_selectable()));
}

#[test]
fn opening_the_menu_clears_the_query() {
    let mut state = MenuState::new();
    state.query.insert("abc");
    state.open();
    assert!(state.query.is_empty());
}

fn trigger_at(x: f32, y: f32) -> Anchor {
    Anchor {
        origin: Point::new(x, y),
        trigger_height: 36.0,
        width: 200.0,
        gap: GAP,
    }
}

#[test]
fn a_menu_opens_below_its_trigger_when_there_is_room() {
    let placed = place(trigger_at(100.0, 100.0), Size::new(800.0, 600.0), 120.0);
    assert!(placed.below);
    assert_eq!(placed.position, Point::new(100.0, 100.0 + 36.0 + GAP));
    assert_eq!(placed.size, Size::new(200.0, 120.0));
}

#[test]
fn a_menu_flips_above_its_trigger_near_the_bottom_edge() {
    let placed = place(trigger_at(100.0, 520.0), Size::new(800.0, 600.0), 200.0);
    assert!(!placed.below);
    assert_eq!(
        placed.position.y + placed.size.height,
        520.0 - GAP,
        "the menu ends a gap above the trigger"
    );
}

#[test]
fn a_menu_shifts_left_at_the_right_edge_and_keeps_its_width() {
    let placed = place(trigger_at(760.0, 100.0), Size::new(800.0, 600.0), 120.0);
    assert_eq!(placed.size.width, 200.0);
    assert_eq!(placed.position.x, 800.0 - SCREEN_MARGIN - 200.0);
}

#[test]
fn a_context_menu_opens_at_the_pointer_with_no_gap() {
    let pointer = Anchor {
        origin: Point::new(300.0, 200.0),
        trigger_height: 0.0,
        width: 200.0,
        gap: 0.0,
    };
    let placed = place(pointer, Size::new(800.0, 600.0), 120.0);
    assert_eq!(placed.position, Point::new(300.0, 200.0));
}

#[test]
fn a_menu_taller_than_the_room_is_capped_so_it_scrolls() {
    let placed = place(trigger_at(100.0, 100.0), Size::new(800.0, 600.0), 900.0);
    assert!(placed.size.height <= MAX_MENU_HEIGHT);
}
