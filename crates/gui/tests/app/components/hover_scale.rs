use super::*;

#[test]
fn a_widget_at_rest_keeps_its_size() {
    assert_eq!(scale_at(0.0, GROWTH), 1.0);
}

#[test]
fn a_fully_hovered_widget_grows_by_the_growth() {
    assert!((scale_at(1.0, GROWTH) - 1.03).abs() < 1e-6);
}

#[test]
fn the_growth_follows_the_progress_and_stays_in_range() {
    assert!((scale_at(0.5, GROWTH) - 1.015).abs() < 1e-6, "halfway at half progress");
    assert_eq!(scale_at(-1.0, GROWTH), 1.0, "below rest stays at rest");
    assert!((scale_at(3.0, GROWTH) - 1.03).abs() < 1e-6, "past full stays at full");
}
