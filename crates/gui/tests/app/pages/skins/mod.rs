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

/// What a drawing of `job` made: a one-pixel picture for each of its skins, in the job's frame.
fn drawn_for(job: &Job) -> Drawn {
    Drawn {
        generation: job.generation,
        frame: job.frame,
        previews: job.items.iter().map(|(slot, _)| (*slot, Handle::from_rgba(1, 1, vec![0; 4]))).collect(),
    }
}

/// A walking page with one saved skin, and the clock at its start, which is returned with it.
fn walking_page() -> (Skins, Instant) {
    let mut page = Skins::default();
    page.update(Message::Pose(Pose::Walk));
    page.set_scan(scan_of(vec![saved("Red", [200, 0, 0, 255])]));
    let start = Instant::now();
    page.update(Message::Tick(start));
    (page, start)
}

/// The moment on the clock that shows frame `frame` of a walk, counted from `start`.
fn walk_moment(start: Instant, frame: usize) -> Instant {
    start + Duration::from_secs_f32(previews::frame_time(Pose::Walk, frame))
}

#[test]
fn a_new_list_asks_for_its_frame_once() {
    let mut page = Skins::default();
    page.set_scan(scan_of(vec![saved("Red", [200, 0, 0, 255])]));
    let job = page.preview_job().expect("a new list asks for its frame");
    assert!(
        job.items.iter().any(|(slot, _)| *slot == Slot::Saved(0)),
        "the saved skin is in the frame"
    );
    page.set_previews(drawn_for(&job));
    assert!(page.preview_job().is_none(), "the frame is drawn once and kept");
}

#[test]
fn a_pose_change_draws_the_frames_of_the_new_pose() {
    let mut page = Skins::default();
    page.set_scan(scan_of(vec![saved("Red", [200, 0, 0, 255])]));
    let job = page.preview_job().expect("the first frame is asked for");
    page.set_previews(drawn_for(&job));
    page.update(Message::Pose(Pose::Run));
    assert!(page.saved_preview(0).is_none(), "the frames of the old pose are not shown");
    let run = page.preview_job().expect("the new pose asks for its frames");
    assert_eq!(run.pose, Pose::Run);
    assert_eq!(run.frame, 0);
}

#[test]
fn a_drawing_made_for_forgotten_frames_is_dropped() {
    let mut page = Skins::default();
    page.set_scan(scan_of(vec![saved("Red", [200, 0, 0, 255])]));
    let old = page.preview_job().expect("the first frame is asked for");
    page.set_scan(scan_of(vec![saved("Red", [200, 0, 0, 255]), saved("Blue", [0, 0, 200, 255])]));
    page.set_previews(drawn_for(&old));
    let current = page.preview_job().expect("the new list asks for its frame");
    assert!(page.saved_preview(0).is_none(), "a drawing made before the list changed is not kept");
    page.set_previews(drawn_for(&current));
    assert!(page.saved_preview(0).is_some(), "the drawing of the new list is kept");
}

#[test]
fn the_frame_on_show_follows_the_clock() {
    let (mut page, start) = walking_page();
    let first = page.preview_job().expect("frame 0 is asked for");
    assert_eq!(first.frame, 0);
    page.set_previews(drawn_for(&first));
    page.update(Message::Tick(walk_moment(start, 3)));
    assert!(page.saved_preview(0).is_some(), "the last frame stands in until frame 3 is drawn");
    let later = page.preview_job().expect("frame 3 is asked for when the clock reaches it");
    assert_eq!(later.frame, 3);
    assert!((later.time - previews::frame_time(Pose::Walk, 3)).abs() < 1e-6);
}

#[test]
fn a_frame_drawn_late_is_kept_for_its_moment_of_the_loop() {
    let (mut page, start) = walking_page();
    let first = page.preview_job().expect("frame 0 is asked for");
    page.update(Message::Tick(walk_moment(start, 2)));
    page.set_previews(drawn_for(&first));
    let second = page.preview_job().expect("frame 2 is asked for next");
    assert_eq!(second.frame, 2);
    page.set_previews(drawn_for(&second));
    page.update(Message::Tick(start + Duration::from_secs(1)));
    assert!(page.preview_job().is_none(), "frame 0, drawn late, is kept for the loop's return to it");
    assert!(page.saved_preview(0).is_some());
}

#[test]
fn one_loop_is_drawn_once_and_the_loop_after_it_draws_nothing() {
    let (mut page, start) = walking_page();
    let frames = previews::frames_per_loop(Pose::Walk);
    let mut drawings = 0;
    for step in 0..2 * frames {
        page.update(Message::Tick(walk_moment(start, step % frames)));
        while let Some(job) = page.preview_job() {
            drawings += 1;
            page.set_previews(drawn_for(&job));
        }
    }
    assert_eq!(drawings, frames, "each frame of the loop is drawn once, and the second loop draws none");
}

#[test]
fn a_drawing_still_running_holds_the_next_one_back() {
    let (mut page, start) = walking_page();
    let first = page.preview_job().expect("the first frame starts");
    page.update(Message::Tick(walk_moment(start, 1)));
    assert!(page.preview_job().is_none(), "no second drawing while the first runs");
    page.set_previews(drawn_for(&first));
    assert!(page.preview_job().is_some(), "frame 1, which is not drawn, starts once the first is done");
}

#[test]
fn a_rescan_of_the_same_skins_keeps_the_frames() {
    let (mut page, _) = walking_page();
    let job = page.preview_job().expect("the frame is asked for");
    page.set_previews(drawn_for(&job));
    page.set_scan(scan_of(vec![saved("Red", [200, 0, 0, 255])]));
    assert!(page.preview_job().is_none(), "the same skins in the same places are drawn already");
    assert!(page.saved_preview(0).is_some(), "and their frame still shows");
}

#[test]
fn only_the_open_sections_are_drawn() {
    let mut page = Skins::default();
    page.set_scan(scan_of(vec![saved("Red", [200, 0, 0, 255])]));
    page.update(Message::ToggleSection(Section::Saved));
    let job = page.preview_job().expect("a change asks for its frame");
    assert!(job.items.iter().all(|(slot, _)| *slot != Slot::Saved(0)), "a folded section is not drawn");
    assert!(
        job.items.iter().any(|(slot, _)| matches!(slot, Slot::Bundled(_))),
        "the open defaults are drawn"
    );
    assert!(
        job.items.iter().all(|(slot, _)| match slot {
            Slot::Bundled(index) => BUNDLED[*index].pack == Pack::Defaults,
            _ => true,
        }),
        "and no folded pack is"
    );
}

#[test]
fn a_previewed_cape_is_saved_only_when_it_differs_from_the_worn_one() {
    let mut page = Skins::default();
    assert_eq!(page.cape_change(Some("worn")), None, "the worn cape is not pending");
    page.update(Message::PickCape(CapePick::Owned("worn".to_owned())));
    assert_eq!(page.cape_change(Some("worn")), None, "previewing the worn cape changes nothing");
    page.update(Message::PickCape(CapePick::Owned("other".to_owned())));
    assert_eq!(page.cape_change(Some("worn")), Some(Some("other".to_owned())));
    page.update(Message::PickCape(CapePick::Off));
    assert_eq!(page.cape_change(Some("worn")), Some(None), "no cape takes the worn one off");
    assert_eq!(page.cape_change(None), None, "with no cape worn, no cape has nothing to take off");
}

#[test]
fn the_viewer_shows_the_previewed_cape_on_the_skin() {
    let mut page = Skins::default();
    page.set_appearance(Appearance {
        skin_png: skin_png([255, 0, 0, 255]),
        cape_png: Some(cape_png([0, 0, 200, 255])),
        slim: false,
    });
    assert!(page.shown_skin().is_some_and(|skin| skin.cape), "the worn cape is shown first");
    page.update(Message::PickCape(CapePick::Off));
    assert!(
        page.shown_skin().is_some_and(|skin| !skin.cape),
        "a previewed no-cape takes it off in the viewer"
    );
    page.reset_cape_pick();
    assert!(page.shown_skin().is_some_and(|skin| skin.cape), "reset shows the worn cape again");
}
