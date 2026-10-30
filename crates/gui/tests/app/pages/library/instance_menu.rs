use super::*;

#[test]
fn the_rows_are_play_edit_reveal_a_separator_then_delete() {
    let rows = items();
    assert_eq!(rows.len(), 5);
    assert!(matches!(rows[3], MenuItem::Separator));
    assert!(matches!(rows[4], MenuItem::Action { danger: true, .. }), "delete is the red row");
}

#[test]
fn every_action_row_maps_to_an_action_and_separators_map_to_none() {
    for (index, row) in items().iter().enumerate() {
        assert_eq!(action_at(index).is_some(), !matches!(row, MenuItem::Separator), "row {index}");
    }
    assert_eq!(action_at(0), Some(Action::Play));
    assert_eq!(action_at(1), Some(Action::Edit));
    assert_eq!(action_at(2), Some(Action::Reveal));
    assert_eq!(action_at(4), Some(Action::Delete));
    assert_eq!(action_at(9), None, "an index past the rows picks nothing");
}
