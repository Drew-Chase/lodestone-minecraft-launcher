use super::*;

fn composed(background: &'static str, symbol: &'static str) -> IconChoice {
    IconChoice::Composed { background, symbol }
}

#[test]
fn picking_a_background_or_symbol_clears_an_upload() {
    let mut editor = IconEditor::new(&IconChoice::Uploaded(Upload::new(vec![1, 2, 3])), Vec::new());
    assert!(matches!(editor.choice(), IconChoice::Uploaded(_)));

    editor.update(Message::Background(1));
    assert_eq!(editor.choice(), composed(BACKGROUNDS[1].id, SYMBOLS[0].id));
}

#[test]
fn saving_returns_the_composition_as_it_stands() {
    let mut editor = IconEditor::new(&composed("rose", "grass"), Vec::new());
    editor.update(Message::Symbol(3));
    assert_eq!(editor.update(Message::Save), Outcome::Save(composed("rose", SYMBOLS[3].id)));
    assert_eq!(
        IconEditor::new(&composed("rose", "grass"), Vec::new()).update(Message::Cancel),
        Outcome::Cancel
    );
}

#[test]
fn a_recent_restores_its_composition() {
    let recents = vec![InstanceIcon::Composed {
        background: "indigo".to_string(),
        symbol: "gold".to_string(),
    }];
    let mut editor = IconEditor::new(&composed("rose", "grass"), recents);
    editor.update(Message::RecentPicked(0));
    assert_eq!(editor.choice(), composed("indigo", "gold"));
}

#[test]
fn the_background_carousel_pages_through_its_swatches() {
    let mut editor = IconEditor::new(&composed("rose", "grass"), Vec::new());
    assert_eq!(editor.background_offset, 0);

    editor.update(Message::BackgroundsStep(Step::Forward));
    assert_eq!(editor.background_offset, 1, "seven backgrounds show six at a time");
    editor.update(Message::BackgroundsStep(Step::Forward));
    assert_eq!(editor.background_offset, 1, "nothing lies past the last page");
    editor.update(Message::BackgroundsStep(Step::Back));
    assert_eq!(editor.background_offset, 0);
}

#[test]
fn symbols_wrap_into_rows_of_six() {
    let mut editor = IconEditor::new(&composed("rose", "grass"), Vec::new());
    assert_eq!(editor.symbol_row(), 0);

    editor.update(Message::Symbol(6));
    assert_eq!(editor.symbol_row(), 1, "the seventh symbol starts the second row");
    editor.update(Message::Symbol(15));
    assert_eq!(editor.symbol_row(), 2, "the last symbol sits on the third row");
}

#[test]
fn a_recent_pages_its_background_into_view_and_its_symbol_into_its_row() {
    let recents = vec![InstanceIcon::Composed {
        background: "midnight".to_string(),
        symbol: "redstone".to_string(),
    }];
    let mut editor = IconEditor::new(&composed("rose", "grass"), recents);
    editor.update(Message::RecentPicked(0));

    assert_eq!(editor.background_offset, 1, "midnight is the seventh background");
    assert_eq!(editor.symbol_row(), 1, "redstone is the eighth symbol, on the second row");
}
