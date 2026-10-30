use super::*;

#[test]
fn picking_a_scene_chooses_it_and_saving_returns_it() {
    let mut editor = BannerEditor::new(&BannerChoice::Scene(0));
    assert_eq!(editor.update(Message::Pick(7)), Outcome::Stay);
    assert_eq!(editor.update(Message::Save), Outcome::Save(BannerChoice::Scene(7)));
}

#[test]
fn an_upload_replaces_the_scene_and_saving_keeps_it() {
    let mut editor = BannerEditor::new(&BannerChoice::Scene(3));
    assert_eq!(editor.update(Message::Upload), Outcome::Upload, "the dialog asks for the image");
    editor.set_upload(Upload::new(vec![1, 2, 3]));
    assert!(matches!(editor.update(Message::Save), Outcome::Save(BannerChoice::Uploaded(_))));
}

#[test]
fn cancelling_leaves_the_banner_as_it_was() {
    let mut editor = BannerEditor::new(&BannerChoice::Scene(3));
    editor.update(Message::Pick(9));
    assert_eq!(editor.update(Message::Cancel), Outcome::Cancel);
}
