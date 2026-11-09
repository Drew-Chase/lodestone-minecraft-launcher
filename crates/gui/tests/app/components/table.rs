use super::*;

const NAME_GAP: f32 = 12.0;

fn columns() -> [Column; 3] {
    [
        Column { width: 120.0, hide: Some(0) },
        Column { width: 100.0, hide: Some(1) },
        Column { width: 80.0, hide: None },
    ]
}

#[test]
fn every_column_shows_when_the_row_has_room_for_all_of_them() {
    // 120 + 100 + 80 fixed, three gaps of 12 and the name's 140: 476px.
    assert_eq!(fit(&columns(), 476.0, NAME_MIN, NAME_GAP), vec![true, true, true]);
}

#[test]
fn the_lowest_hide_order_goes_first() {
    // One pixel short of the full row, so the first column with a hide order goes.
    assert_eq!(fit(&columns(), 475.0, NAME_MIN, NAME_GAP), vec![false, true, true]);
    // 343 is short of the row with the first column gone (344px), so the second goes too.
    assert_eq!(fit(&columns(), 343.0, NAME_MIN, NAME_GAP), vec![false, false, true]);
}

#[test]
fn a_column_without_a_hide_order_never_hides() {
    assert_eq!(fit(&columns(), 0.0, NAME_MIN, NAME_GAP), vec![false, false, true]);
}

#[test]
fn an_empty_table_fits_trivially() {
    assert!(fit(&[], 100.0, NAME_MIN, NAME_GAP).is_empty());
}
