use super::*;

#[test]
fn text_becomes_a_text_slot() {
    assert!(matches!(Slot::<()>::from("Title"), Slot::Text(Cow::Borrowed("Title"))));
    assert!(matches!(Slot::<()>::from(String::from("Title")), Slot::Text(Cow::Owned(_))));
}

#[test]
fn widgets_become_element_slots() {
    assert!(matches!(Slot::<()>::from(iced::widget::row![]), Slot::Element(_)));
    assert!(matches!(Slot::<()>::from(iced::widget::column![]), Slot::Element(_)));
    assert!(matches!(Slot::<()>::from(container(iced::widget::row![])), Slot::Element(_)));
    assert!(matches!(Slot::<()>::from(Element::from(iced::widget::row![])), Slot::Element(_)));
}

#[test]
fn a_styled_text_keeps_its_own_style() {
    assert!(matches!(Slot::<()>::from(Text::new("Styled")), Slot::Element(_)));
}

#[test]
fn defaults_have_no_footer_close_or_icon() {
    let modal = Modal::<()>::new("Title", "Body");
    assert_eq!(modal.width, DEFAULT_WIDTH);
    assert!(modal.close_on_scrim);
    assert!(modal.footer.is_none());
    assert!(modal.on_close.is_none());
    assert!(modal.icon.is_none());
    assert!(modal.subtitle.is_none());
}

#[test]
fn builders_set_every_option() {
    let modal = Modal::new("Title", "Body")
        .subtitle("Subtitle")
        .icon("<svg/>", Color::WHITE, Color::BLACK)
        .footer("Footer")
        .on_close(7)
        .close_on_scrim(false)
        .width(640.0);
    assert_eq!(modal.subtitle.as_deref(), Some("Subtitle"));
    assert_eq!(
        modal.icon,
        Some(HeaderIcon {
            svg: "<svg/>",
            ink: Color::WHITE,
            tint: Color::BLACK,
        })
    );
    assert!(matches!(modal.footer, Some(Slot::Text(_))));
    assert_eq!(modal.on_close, Some(7));
    assert!(!modal.close_on_scrim);
    assert_eq!(modal.width, 640.0);
}

#[test]
fn builds_into_an_element() {
    let _: Element<'_, ()> = Modal::new("Title", iced::widget::row![])
        .footer(iced::widget::column![])
        .on_close(())
        .into();
}
