use super::*;

#[test]
fn visual_tracks_knob_position() {
    let off = Toggle::<()>::new(false).visual(Status::OffIdle);
    let on = Toggle::<()>::new(true).visual(Status::OnIdle);
    assert_eq!(off.knob, 0.0);
    assert_eq!(on.knob, 1.0);
    assert_ne!(off.track, on.track);
    assert_ne!(off.knob_color, on.knob_color);
}

#[test]
fn status_tracks_the_on_state() {
    let on = Toggle::<()>::new(true);
    let off = Toggle::<()>::new(false);
    assert_eq!(on.status_for(false, false), Status::OnIdle);
    assert_eq!(off.status_for(false, false), Status::OffIdle);
    assert_eq!(on.status_for(true, true), Status::OnPressed);
    assert_eq!(on.status_for(false, true), Status::OnHovered);
    assert_eq!(off.status_for(false, true), Status::OffHovered);
}

#[test]
fn disabled_status_ignores_hover_but_keeps_the_value() {
    let on = Toggle::<()>::new(true).disabled(true);
    let off = Toggle::<()>::new(false).disabled(true);
    assert_eq!(on.status_for(false, true), Status::Disabled(true));
    assert_eq!(off.status_for(true, true), Status::Disabled(false));
}

#[test]
fn hover_brightens_and_press_dims() {
    let hovered = Toggle::<()>::new(true).visual(Status::OnHovered);
    let pressed = Toggle::<()>::new(true).visual(Status::OnPressed);
    assert_eq!(hovered.track, iced::color!(0x2AFF8C));
    assert_eq!(pressed.track, color::Primary::DIM);
}

#[test]
fn disabled_flips_still_change_the_visual() {
    let on = Toggle::<()>::new(true).disabled(true).visual(Status::Disabled(true));
    let off = Toggle::<()>::new(false).disabled(true).visual(Status::Disabled(false));
    assert_ne!(on.track, off.track);
    assert_ne!(on.knob, off.knob);
}
