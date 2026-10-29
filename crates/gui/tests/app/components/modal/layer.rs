use super::*;

#[test]
fn card_height_counts_one_rule_per_divider() {
    assert_eq!(card_height(80.0, 200.0, None), 281.0);
    assert_eq!(card_height(80.0, 200.0, Some(70.0)), 352.0);
}

#[test]
fn the_body_gets_what_the_header_and_footer_leave() {
    assert_eq!(body_room(600.0, 80.0, Some(70.0)), 448.0);
    assert_eq!(body_room(600.0, 80.0, None), 519.0);
}

#[test]
fn the_body_never_gets_negative_room() {
    assert_eq!(body_room(100.0, 80.0, Some(70.0)), 0.0);
}

#[test]
fn a_footer_is_the_third_child() {
    let with = Layer::<()>::new(
        iced::widget::row![].into(),
        iced::widget::row![].into(),
        Some(iced::widget::row![].into()),
        560.0,
        None,
        true,
    );
    let without = Layer::<()>::new(iced::widget::row![].into(), iced::widget::row![].into(), None, 560.0, None, true);
    assert!(with.has_footer());
    assert!(!without.has_footer());
}
