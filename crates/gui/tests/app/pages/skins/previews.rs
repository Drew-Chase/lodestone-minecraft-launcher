use super::*;

/// A PNG of a `width` by `height` image, filled with one colour.
fn png(width: u32, height: u32, color: [u8; 4]) -> Vec<u8> {
    let image = image::RgbaImage::from_pixel(width, height, image::Rgba(color));
    let mut bytes = std::io::Cursor::new(Vec::new());
    image.write_to(&mut bytes, image::ImageFormat::Png).expect("encodes");
    bytes.into_inner()
}

/// A skin's item, as the page builds it for `png`: its texels, and the model it wears.
fn item(png: &[u8], slim: bool) -> Item {
    Item {
        texels: skin_texels(png).map(Arc::new),
        slim,
    }
}

/// A job that draws `items` in the standing pose at `time`, as frame 0, with no cape.
fn job(items: Vec<(Slot, Item)>, time: f32) -> Job {
    Job {
        generation: 1,
        frame: 0,
        pose: Pose::Stand,
        time,
        layers: Layers::default(),
        cape: None,
        items,
    }
}

/// The pixels of a drawn preview, or `None` when there is none.
fn pixels(preview: Option<&Handle>) -> Option<&[u8]> {
    match preview? {
        Handle::Rgba { pixels, .. } => Some(&pixels[..]),
        Handle::Path(..) | Handle::Bytes(..) => None,
    }
}

/// The preview a drawing made in `slot`, when it made one.
fn preview(drawn: &Drawn, slot: Slot) -> Option<&Handle> {
    drawn
        .previews
        .iter()
        .find(|(drawn_slot, _)| *drawn_slot == slot)
        .map(|(_, handle)| handle)
}

#[test]
fn each_skin_is_drawn_in_its_slot_and_one_that_cannot_be_drawn_has_no_preview() {
    let drawn = draw(job(
        vec![
            (Slot::Saved(0), item(&png(64, 64, [200, 0, 0, 255]), false)),
            (Slot::Saved(2), item(b"not a picture", false)),
            (Slot::Archived(0), item(&png(64, 64, [0, 0, 200, 255]), true)),
            (Slot::Bundled(5), Item { texels: None, slim: false }),
        ],
        0.0,
    ));
    assert_eq!(drawn.generation, 1, "the drawing answers with the job's generation");
    assert_eq!(drawn.frame, 0, "and with the job's frame");
    assert!(preview(&drawn, Slot::Saved(0)).is_some(), "a skin the viewer can draw has a preview");
    assert!(preview(&drawn, Slot::Saved(2)).is_none(), "bytes that are not a skin have none");
    assert!(preview(&drawn, Slot::Archived(0)).is_some());
    assert!(preview(&drawn, Slot::Bundled(5)).is_some(), "a bundled skin is drawn from the catalog");
    assert_eq!(drawn.previews.len(), 3);
}

#[test]
fn a_worn_cape_shows_on_the_preview() {
    let plain = job(vec![(Slot::Saved(0), item(&png(64, 64, [200, 0, 0, 255]), false))], 0.0);
    let caped = Job {
        cape: Some(png(64, 32, [0, 255, 0, 255])),
        ..job(vec![(Slot::Saved(0), item(&png(64, 64, [200, 0, 0, 255]), false))], 0.0)
    };
    let (plain, caped) = (draw(plain), draw(caped));
    // The skin is red and the cape green, so the green pixels are the cape's.
    let green = |drawn: &Drawn| {
        pixels(preview(drawn, Slot::Saved(0)))
            .unwrap_or_default()
            .chunks_exact(4)
            .filter(|px| px[1] > 150 && px[0] < 80 && px[2] < 80)
            .count()
    };
    assert_eq!(green(&plain), 0, "a skin with no cape has no green");
    assert!(green(&caped) > 0, "the cape the account wears shows on the preview");
}

#[test]
fn a_later_moment_of_the_pose_is_drawn_differently() {
    let walk = |time: f32| Job {
        pose: Pose::Walk,
        ..job(vec![(Slot::Saved(0), item(&png(64, 64, [200, 0, 0, 255]), false))], time)
    };
    let rest = draw(walk(0.0));
    let stride = draw(walk(Pose::Walk.preview_time()));
    assert_ne!(
        pixels(preview(&rest, Slot::Saved(0))),
        pixels(preview(&stride, Slot::Saved(0))),
        "the limbs are further through the stride at the later moment"
    );
}

#[test]
fn a_skin_drawn_with_others_is_drawn_as_it_is_alone() {
    let red = item(&png(64, 64, [200, 0, 0, 255]), false);
    let blue = item(&png(64, 64, [0, 0, 200, 255]), false);
    let slim = item(&png(64, 64, [0, 200, 0, 255]), true);
    let together = draw(job(
        vec![(Slot::Saved(0), red.clone()), (Slot::Saved(1), blue), (Slot::Archived(0), slim.clone())],
        0.25,
    ));
    let red_alone = draw(job(vec![(Slot::Saved(0), red)], 0.25));
    let slim_alone = draw(job(vec![(Slot::Archived(0), slim)], 0.25));
    assert_eq!(
        pixels(preview(&together, Slot::Saved(0))),
        pixels(preview(&red_alone, Slot::Saved(0))),
        "a classic skin drawn with others is drawn as it is alone"
    );
    assert_eq!(
        pixels(preview(&together, Slot::Archived(0))),
        pixels(preview(&slim_alone, Slot::Archived(0))),
        "a slim skin drawn with others is drawn as it is alone"
    );
}

#[test]
fn a_loop_is_cached_in_frames_that_wrap_round_for_every_pose() {
    for pose in Pose::ALL {
        let frames = frames_per_loop(pose);
        assert!((1..=MAX_FRAMES_PER_LOOP).contains(&frames), "{pose:?} is cached in {frames} frames");
        assert_eq!(frame_at(pose, 0.0), 0);
        assert_eq!(frame_at(pose, pose.loop_seconds()), 0, "{pose:?}: the loop ends where it began");
        assert_eq!(frame_at(pose, frame_time(pose, 7)), 7, "{pose:?}: a frame's own moment shows that frame");
    }
}

#[test]
fn a_walk_and_a_run_are_cached_one_frame_per_step_and_a_slow_pose_is_capped() {
    assert_eq!(frames_per_loop(Pose::Walk), 15);
    assert_eq!(frames_per_loop(Pose::Run), 9);
    assert_eq!(
        frames_per_loop(Pose::Stand),
        MAX_FRAMES_PER_LOOP,
        "a slow pose costs no more memory than a walk"
    );
}
