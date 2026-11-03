use super::*;

fn rail() -> NavRail<()> {
    NavRail::new([NavItem::new("<svg/>"), NavItem::new("<svg/>")]).on_select(|_| ())
}

#[test]
fn icon_colors_follow_state() {
    let rail = rail();
    assert_eq!(rail.icon_color(0, None), color::primary());
    assert_eq!(rail.icon_color(1, None), color::text_placeholder());
    assert_eq!(rail.icon_color(1, Some(1)), color::text_headline());
}

#[test]
fn without_on_select_is_not_interactive() {
    let no_callback: NavRail<()> = NavRail::new([NavItem::new("<svg/>")]);
    assert!(!no_callback.interactive());
    assert!(rail().interactive());
}

#[test]
fn intrinsic_size_accounts_for_gaps() {
    let single: NavRail<()> = NavRail::new([NavItem::new("<svg/>")]);
    assert_eq!(single.intrinsic_size().height, ITEM_SIZE);
    let pair = rail();
    assert_eq!(pair.intrinsic_size().height, ITEM_SIZE * 2.0 + GAP);
    assert_eq!(pair.intrinsic_size().width, INDICATOR_INSET + ITEM_SIZE);
}

#[test]
fn a_labelled_slot_keeps_its_label_and_an_unlabelled_one_has_none() {
    assert_eq!(NavItem::labelled("<svg/>", "Library").label, Some("Library"));
    assert_eq!(NavItem::new("<svg/>").label, None);
}

#[test]
fn a_tooltip_sits_right_of_its_slot_and_centred_on_it() {
    let slot = Rectangle::new(Point::new(20.0, 100.0), Size::new(ITEM_SIZE, ITEM_SIZE));
    let bubble = Size::new(90.0, 26.0);
    let origin = tooltip_origin(slot, bubble, Size::new(1280.0, 720.0));
    assert_eq!(origin.x, 20.0 + ITEM_SIZE + TOOLTIP_GAP);
    assert_eq!(origin.y, slot.center_y() - bubble.height / 2.0);
}

#[test]
fn a_tooltip_near_the_window_edge_stays_inside_the_window() {
    let window = Size::new(1280.0, 720.0);
    let slot = Rectangle::new(Point::new(1270.0, 700.0), Size::new(ITEM_SIZE, ITEM_SIZE));
    let bubble = Size::new(90.0, 26.0);
    let origin = tooltip_origin(slot, bubble, window);
    assert!(origin.x >= 0.0 && origin.y >= 0.0, "{origin:?}");
    assert!(origin.x + bubble.width <= window.width, "{origin:?}");
    assert!(origin.y + bubble.height <= window.height, "{origin:?}");
}
