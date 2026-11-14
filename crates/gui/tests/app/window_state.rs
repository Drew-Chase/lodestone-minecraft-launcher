use super::*;

fn saved(maximized: bool, width: u32, height: u32, x: Option<i32>, y: Option<i32>) -> WindowState {
    WindowState {
        maximized,
        width,
        height,
        x,
        y,
    }
}

#[test]
fn no_saved_state_opens_the_default_window_centred() {
    let opening = Opening::from_saved(None);
    assert_eq!(opening, Opening::default());
    assert_eq!(opening.size, DEFAULT_SIZE);
    assert!(!opening.maximized);
    assert!(matches!(opening.position(), Position::Centered));
}

#[test]
fn a_saved_state_restores_size_maximized_and_position() {
    let opening = Opening::from_saved(Some(saved(true, 1600, 900, Some(40), Some(60))));
    assert_eq!(opening.size, Size::new(1600.0, 900.0));
    assert!(opening.maximized);
    assert_eq!(opening.position, Some(Point::new(40.0, 60.0)));
    assert!(matches!(opening.position(), Position::Specific(point) if point == Point::new(40.0, 60.0)));
}

#[test]
fn a_size_below_the_minimum_opens_at_the_minimum() {
    let opening = Opening::from_saved(Some(saved(false, 300, 200, None, None)));
    assert_eq!(opening.size, MIN_SIZE);

    let opening = Opening::from_saved(Some(saved(false, 0, 0, None, None)));
    assert_eq!(opening.size, MIN_SIZE);
}

#[test]
fn an_oversized_dimension_is_clamped() {
    let opening = Opening::from_saved(Some(saved(false, u32::MAX, 900, None, None)));
    assert_eq!(opening.size, Size::new(MAX_DIMENSION as f32, 900.0));
}

#[test]
fn a_position_needs_both_coordinates() {
    assert_eq!(Opening::from_saved(Some(saved(false, 1280, 720, Some(10), None))).position, None);
    assert_eq!(Opening::from_saved(Some(saved(false, 1280, 720, None, Some(10)))).position, None);
}

#[test]
fn a_negative_position_is_kept_for_a_screen_left_of_the_primary() {
    let opening = Opening::from_saved(Some(saved(false, 1280, 720, Some(-1920), Some(120))));
    assert_eq!(opening.position, Some(Point::new(-1920.0, 120.0)));
}

#[test]
fn a_window_without_a_reported_place_saves_none_for_it() {
    let state = to_saved(
        false,
        Geometry {
            size: Size::new(1000.0, 700.0),
            position: None,
        },
    );
    assert_eq!(state, saved(false, 1000, 700, None, None));
}

#[test]
fn fractional_geometry_rounds_to_whole_pixels_when_saved() {
    let state = to_saved(
        true,
        Geometry {
            size: Size::new(1000.4, 699.6),
            position: Some(Point::new(-10.5, 20.2)),
        },
    );
    assert_eq!(state, saved(true, 1000, 700, Some(-11), Some(20)));
}

#[test]
fn a_saved_state_round_trips_through_the_geometry() {
    let original = saved(true, 1400, 820, Some(-1920), Some(120));
    let opening = Opening::from_saved(Some(original));
    assert_eq!(to_saved(opening.maximized, opening.geometry()), original);
}
