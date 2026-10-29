use super::*;

fn dropdown() -> Dropdown<()> {
    Dropdown::new("<svg/>")
        .item(MenuItem::Action {
            label: String::from("Open"),
            icon: None,
            danger: false,
        })
        .separator()
        .item(MenuItem::Action {
            label: String::from("Delete"),
            icon: None,
            danger: true,
        })
        .on_select(|_| ())
}

#[test]
fn builders_accumulate_items() {
    let dropdown = dropdown();
    assert_eq!(dropdown.items.len(), 3);
    assert!(matches!(dropdown.items[1], MenuItem::Separator));
}

#[test]
fn without_on_select_is_not_interactive() {
    let no_callback: Dropdown<()> = Dropdown::new("<svg/>").items([MenuItem::Separator]);
    assert!(!no_callback.interactive());
    assert!(dropdown().interactive());
}
