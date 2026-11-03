use super::*;

/// A PNG of a 64×64 image filled with one colour.
fn skin_png(color: [u8; 4]) -> Vec<u8> {
    let image = image::RgbaImage::from_pixel(64, 64, image::Rgba(color));
    let mut bytes = std::io::Cursor::new(Vec::new());
    image.write_to(&mut bytes, image::ImageFormat::Png).expect("encodes");
    bytes.into_inner()
}

/// A PNG of a 64×32 cape filled with one colour.
fn cape_png(color: [u8; 4]) -> Vec<u8> {
    let image = image::RgbaImage::from_pixel(64, 32, image::Rgba(color));
    let mut bytes = std::io::Cursor::new(Vec::new());
    image.write_to(&mut bytes, image::ImageFormat::Png).expect("encodes");
    bytes.into_inner()
}

/// A saved skin of one colour, named `name`.
fn saved(name: &str, color: [u8; 4]) -> SavedSkin {
    SavedSkin::new(name.to_string(), PathBuf::from(format!("{name}.png")), skin_png(color), None)
}

/// What the folders hold, with no archive and no bundled digests.
fn scan_of(saved: Vec<SavedSkin>) -> Scan {
    Scan {
        saved,
        archived: Vec::new(),
        bundled: Vec::new(),
    }
}

#[test]
fn the_packs_start_folded_away_except_the_defaults() {
    let page = Skins::default();
    assert!(!page.collapsed(Section::Saved));
    assert!(!page.collapsed(Section::Archived));
    assert!(!page.collapsed(Section::Pack(Pack::Defaults)));
    assert!(page.collapsed(Section::Pack(Pack::Minecon2017)));
}

#[test]
fn toggling_a_section_folds_it_and_unfolds_it_again() {
    let mut page = Skins::default();
    page.update(Message::ToggleSection(Section::Saved));
    assert!(page.collapsed(Section::Saved));
    page.update(Message::ToggleSection(Section::Saved));
    assert!(!page.collapsed(Section::Saved));
}

#[test]
fn a_saved_skin_is_used_in_the_model_the_switch_picks() {
    let mut page = Skins::default();
    page.set_scan(scan_of(vec![saved("Red", [200, 0, 0, 255])]));
    page.update(Message::Choose(Choice::Saved(0)));
    assert_eq!(page.choice(), Choice::Saved(0));
    assert!(page.shown_skin().is_some());
    assert!(!page.shown_slim(), "the switch starts on the classic model");
    page.update(Message::Slim(true));
    assert!(page.shown_slim());
    let chosen = page.chosen().expect("a saved skin can be used");
    assert_eq!(chosen.name, "Red");
    assert!(chosen.slim, "the chosen model goes with the upload");
}

#[test]
fn the_current_skin_has_nothing_to_use() {
    let page = Skins::default();
    assert_eq!(page.choice(), Choice::Current);
    assert!(page.chosen().is_none());
}

#[test]
fn a_choice_that_no_longer_names_a_skin_goes_back_to_the_current_one() {
    let mut page = Skins::default();
    page.set_scan(scan_of(vec![saved("Red", [200, 0, 0, 255])]));
    page.update(Message::Choose(Choice::Saved(0)));
    page.set_scan(scan_of(Vec::new()));
    assert_eq!(page.choice(), Choice::Current);
}

#[test]
fn the_current_skin_is_shown_with_the_cape_the_account_wears() {
    let mut page = Skins::default();
    page.set_appearance(Appearance {
        skin_png: skin_png([0, 200, 0, 255]),
        cape_png: Some(cape_png([0, 0, 200, 255])),
        slim: true,
    });
    let shown = page.shown_skin().expect("the current skin is shown");
    assert!(shown.cape, "the cape the account wears is on the viewer's skin");
    assert!(page.shown_slim(), "the account's own model is shown for the current skin");
}

#[test]
fn the_known_digests_are_the_saved_skins_and_the_bundled_ones() {
    let mut page = Skins::default();
    page.set_scan(Scan {
        saved: vec![saved("Red", [200, 0, 0, 255])],
        archived: Vec::new(),
        bundled: vec!["bundled".to_string()],
    });
    assert!(page.known().contains(&"bundled".to_string()));
    assert!(page.known().contains(&library::digest(&skin_png([200, 0, 0, 255]))));
}

#[test]
fn an_archived_skin_is_used_in_the_model_it_was_worn_in() {
    let mut page = Skins::default();
    let archived = SavedSkin::new(
        "2 Nov 2026".to_string(),
        PathBuf::from("digest-slim.png"),
        skin_png([0, 0, 200, 255]),
        Some(true),
    );
    page.set_scan(Scan {
        saved: Vec::new(),
        archived: vec![archived],
        bundled: Vec::new(),
    });
    page.update(Message::Choose(Choice::Archived(0)));
    assert!(page.shown_slim(), "the model the skin was worn in");
    assert!(page.chosen().expect("an archived skin can be used").slim);
    assert_eq!(page.archived_path(0), Some(PathBuf::from("digest-slim.png")));
}

#[test]
fn the_pose_and_layer_switches_change_what_the_viewer_shows() {
    let mut page = Skins::default();
    page.update(Message::Pose(Pose::Run));
    assert_eq!(page.pose(), Pose::Run);
    assert!(page.layers().hat);
    page.update(Message::Layer(Layer::Hat));
    assert!(!page.layers().hat);
    page.update(Message::Layer(Layer::Hat));
    assert!(page.layers().hat);
}

#[test]
fn a_busy_page_says_so_until_the_change_ends() {
    let mut page = Skins::default();
    assert!(!page.busy());
    page.set_busy(true);
    assert!(page.busy());
    page.set_busy(false);
    assert!(!page.busy());
}

#[test]
fn a_change_of_list_or_pose_asks_for_the_previews_again_once() {
    let mut page = Skins::default();
    page.set_scan(scan_of(vec![saved("Red", [200, 0, 0, 255])]));
    assert!(page.preview_job().is_some(), "a new list asks for previews");
    page.set_previews(1, Previews::default());
    assert!(page.preview_job().is_none(), "the previews are asked for once");
    page.update(Message::Pose(Pose::Run));
    assert!(page.preview_job().is_some(), "a new pose asks for them again");
    page.set_previews(2, Previews::default());
    page.update(Message::Choose(Choice::Saved(0)));
    assert!(page.preview_job().is_none(), "choosing a skin changes no preview");
}

#[test]
fn a_frame_asks_for_a_drawing_at_its_moment_of_the_poses_motion() {
    let mut page = Skins::default();
    page.set_scan(scan_of(vec![saved("Red", [200, 0, 0, 255])]));
    let first = page.preview_job().expect("the list asks for a drawing");
    page.set_previews(first.generation, Previews::default());
    let start = Instant::now();
    page.update(Message::Tick(start));
    page.update(Message::Tick(start + Duration::from_millis(500)));
    let frame = page.preview_job().expect("a frame falls due");
    assert!((frame.time - 0.5).abs() < 1e-3, "the drawing is at half a second: {}", frame.time);
    assert!(frame.generation > first.generation);
}

#[test]
fn a_drawing_still_running_holds_the_next_one_back() {
    let mut page = Skins::default();
    page.set_scan(scan_of(vec![saved("Red", [200, 0, 0, 255])]));
    let first = page.preview_job().expect("the first drawing starts");
    page.update(Message::Tick(Instant::now()));
    assert!(page.preview_job().is_none(), "no second drawing while the first runs");
    page.set_previews(first.generation, Previews::default());
    assert!(page.preview_job().is_some(), "the frame due meanwhile starts once the first is done");
}

#[test]
fn a_drawing_that_is_not_the_last_one_asked_for_is_not_shown() {
    let mut page = Skins::default();
    page.set_scan(scan_of(vec![saved("Red", [200, 0, 0, 255])]));
    let first = page.preview_job().expect("the first drawing starts");
    let handle = Handle::from_rgba(1, 1, vec![0; 4]);
    page.set_previews(
        first.generation,
        Previews {
            saved: vec![Some(handle.clone())],
            ..Previews::default()
        },
    );
    assert!(page.saved_preview(0).is_some(), "the drawing asked for is shown");
    page.set_previews(first.generation - 1, Previews::default());
    assert!(page.saved_preview(0).is_some(), "an older drawing does not replace a newer one");
}
