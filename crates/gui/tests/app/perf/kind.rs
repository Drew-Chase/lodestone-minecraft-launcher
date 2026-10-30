use super::*;

#[test]
fn top_level_messages_are_named_after_their_variant() {
    assert_eq!(message_kind(&LodestoneAppMessage::NavBack), "update.NavBack");
    assert_eq!(message_kind(&LodestoneAppMessage::AuthProbeTick), "update.AuthProbeTick");
}

#[test]
fn library_messages_are_named_under_the_library() {
    let scrolled = LodestoneAppMessage::Library(LibraryMessage::Scrolled(120.0));
    assert_eq!(message_kind(&scrolled), "update.Library.Scrolled");

    let hover = LodestoneAppMessage::Library(LibraryMessage::Hover {
        instance: "survival".into(),
        hovered: true,
    });
    assert_eq!(message_kind(&hover), "update.Library.Hover");
}
