use super::*;

#[test]
fn the_pair_is_centred_when_there_is_room() {
    // 1000 wide, a 300 main segment and a 200 side segment: the pair's left edge is 350, centring the main segment.
    assert_eq!(main_left(1000.0, 300.0, 200.0), Some(350.0));
}

#[test]
fn the_pair_shifts_left_so_the_side_segment_keeps_its_edge() {
    // Centred, the side would end at 150 + 400 + 15 + 200 = 765 in a 700 window. Shifted, it ends EDGE short of the right.
    let left = main_left(700.0, 400.0, 200.0).expect("fits");
    assert_eq!(left, 70.0);
    assert_eq!(left + 400.0 + GAP + 200.0, 700.0 - EDGE);
}

#[test]
fn the_pair_is_refused_when_it_cannot_keep_the_edge_either_side() {
    // The pair is 615 wide, so it needs 645 with the edges; 600 is too narrow.
    assert_eq!(main_left(600.0, 400.0, 200.0), None);
}

#[test]
fn the_pair_fits_exactly_at_the_edge_width() {
    assert_eq!(main_left(645.0, 400.0, 200.0), Some(EDGE));
}
