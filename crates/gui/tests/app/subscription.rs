use super::*;

fn press(button: mouse::Button) -> iced::Event {
    iced::Event::Mouse(mouse::Event::ButtonPressed(button))
}

#[test]
fn side_buttons_map_to_history_navigation() {
    let window_id = window::Id::unique();

    assert!(matches!(
        LodestoneLauncher::mouse_navigation(press(mouse::Button::Back), event::Status::Ignored, window_id),
        Some(LodestoneAppMessage::NavBack)
    ));
    assert!(matches!(
        LodestoneLauncher::mouse_navigation(press(mouse::Button::Forward), event::Status::Ignored, window_id),
        Some(LodestoneAppMessage::NavForward)
    ));
}

#[test]
fn captured_events_and_other_input_are_ignored() {
    let window_id = window::Id::unique();

    // A widget consumed the press — it belongs to the widget.
    assert!(LodestoneLauncher::mouse_navigation(press(mouse::Button::Back), event::Status::Captured, window_id).is_none());
    // Ordinary buttons and non-button events never navigate.
    assert!(LodestoneLauncher::mouse_navigation(press(mouse::Button::Left), event::Status::Ignored, window_id).is_none());
    assert!(
        LodestoneLauncher::mouse_navigation(
            iced::Event::Mouse(mouse::Event::CursorMoved {
                position: iced::Point::ORIGIN
            }),
            event::Status::Ignored,
            window_id
        )
        .is_none()
    );
}
