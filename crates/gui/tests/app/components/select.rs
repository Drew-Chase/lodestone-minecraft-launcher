use super::*;

/// The badge sits after the label, so the label's measured width must grow with its text.
#[test]
fn the_label_width_grows_with_its_text() {
    let short = text_width("1", 13.0, fonts::Inter::MEDIUM);
    let version = text_width("1.21.4", 13.0, fonts::Inter::MEDIUM);
    assert!(short > 0.0 && version > short, "short {short}, version {version}");
}
