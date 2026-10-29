use super::*;

#[test]
fn focus_ring_is_green() {
    let focused = style(&Theme::Dark, text_input::Status::Focused { is_hovered: false });
    assert_eq!(focused.border.color, rgb!(34, 255, 132, 50%));
    assert_eq!(focused.background, Background::Color(rgb!(34, 255, 132, 3%)));
}

#[test]
fn disabled_uses_disabled_ink() {
    let disabled = style(&Theme::Dark, text_input::Status::Disabled);
    assert_eq!(disabled.value, color::Text::DISABLED);
    assert_eq!(disabled.placeholder, color::Text::DISABLED);
}

#[test]
fn idle_matches_the_spec() {
    let active = style(&Theme::Dark, text_input::Status::Active);
    assert_eq!(active.border.radius, 12.0.into());
    assert_eq!(active.background, Background::Color(rgb!(255, 255, 255, 4%)));
}
