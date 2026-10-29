use super::*;

#[test]
fn each_kind_has_its_default_title_and_buttons() {
    let cases = [
        (Alert::info("m"), Kind::Info, "Info", Buttons::Close),
        (Alert::warn("m"), Kind::Warning, "Warning", Buttons::OkCancel),
        (Alert::error("m"), Kind::Error, "Error", Buttons::Close),
        (Alert::danger("m"), Kind::Danger, "Are you sure?", Buttons::YesNo),
    ];
    for (alert, kind, title, buttons) in cases {
        assert_eq!(alert.kind(), kind);
        assert_eq!(alert.heading(), title);
        assert_eq!(alert.button_set(), buttons);
        assert_eq!(alert.message(), "m");
    }
}

#[test]
fn kinds_have_distinct_glyph_palettes() {
    let inks = [Kind::Info, Kind::Warning, Kind::Error, Kind::Danger].map(|kind| kind.palette().0);
    for (index, ink) in inks.iter().enumerate() {
        assert!(!inks[index + 1..].contains(ink), "{ink:?} is shared");
    }
}

#[test]
fn overrides_replace_the_defaults() {
    let alert = Alert::danger("Delete it?").title("Delete instance").ok_cancel();
    assert_eq!(alert.heading(), "Delete instance");
    assert_eq!(alert.button_set(), Buttons::OkCancel);
    assert_eq!(Alert::info("m").yes_no().button_set(), Buttons::YesNo);
    assert_eq!(Alert::warn("m").close().button_set(), Buttons::Close);
}

#[test]
fn owned_messages_are_accepted() {
    let name = "Aether Legacy";
    let alert = Alert::danger(format!("Delete “{name}”?"));
    assert_eq!(alert.message(), "Delete “Aether Legacy”?");
}

#[test]
fn button_labels_pair_affirmative_with_negative() {
    assert_eq!((Buttons::YesNo.affirmative(), Buttons::YesNo.negative()), ("Yes", Some("No")));
    assert_eq!((Buttons::OkCancel.affirmative(), Buttons::OkCancel.negative()), ("Okay", Some("Cancel")));
    assert_eq!((Buttons::Close.affirmative(), Buttons::Close.negative()), ("Close", None));
}

#[test]
fn only_danger_paints_the_affirmative_button_red() {
    assert_eq!(Alert::danger("m").affirmative_variant(), Variant::Danger);
    assert_eq!(Alert::warn("m").affirmative_variant(), Variant::Primary);
    assert_eq!(Alert::info("m").affirmative_variant(), Variant::Primary);
    assert_eq!(Alert::error("m").affirmative_variant(), Variant::Primary);
}

#[test]
fn every_kind_and_button_set_builds_a_view() {
    for alert in [Alert::info("m"), Alert::warn("m"), Alert::error("m"), Alert::danger("m")] {
        for buttons in [Buttons::YesNo, Buttons::OkCancel, Buttons::Close] {
            let _ = alert.clone().buttons(buttons).view();
        }
    }
}
