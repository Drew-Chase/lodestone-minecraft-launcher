use super::*;

#[test]
fn defaults_match_the_guide() {
    let dot = PulseDot::new();
    assert_eq!(dot.size, 8.0);
    assert_eq!(dot.color, color::Primary::DEFAULT);
    assert!(dot.pulsing);
}
