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
