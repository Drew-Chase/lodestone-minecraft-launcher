use super::*;

#[test]
fn fraction_is_clamped() {
    let bar = ProgressBar::new(1.4);
    assert_eq!(bar.fraction, 1.0);
    let bar = ProgressBar::new(-0.2);
    assert_eq!(bar.fraction, 0.0);
}
