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
