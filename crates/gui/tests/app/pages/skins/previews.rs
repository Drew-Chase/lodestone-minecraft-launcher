use super::*;

/// A PNG of a 64×64 image filled with one colour.
fn skin_png(color: [u8; 4]) -> Vec<u8> {
    let image = image::RgbaImage::from_pixel(64, 64, image::Rgba(color));
    let mut bytes = std::io::Cursor::new(Vec::new());
    image.write_to(&mut bytes, image::ImageFormat::Png).expect("encodes");
    bytes.into_inner()
}

/// A saved skin's item for `png`, as the page builds it: the skin's texels, and its model.
fn item(png: &[u8], slim: bool) -> Item {
    Item {
        texels: skin_texels(png).map(Arc::new),
        slim,
    }
}

/// A drawing of `saved` skins and nothing else, in the standing pose at `time`.
fn job(saved: Vec<(usize, Item)>, time: f32) -> Job {
    Job {
        generation: 1,
        pose: Pose::Stand,
        time,
        layers: Layers::default(),
        cape: None,
        saved_len: 3,
        archived_len: 0,
        saved,
        archived: Vec::new(),
        bundled: Vec::new(),
    }
}

#[test]
fn each_skin_is_drawn_in_its_place_and_one_that_cannot_be_drawn_has_no_preview() {
    let mut job = job(
        vec![
            (0, item(&skin_png([200, 0, 0, 255]), false)),
            (2, item(b"not a picture", false)),
        ],
        0.0,
    );
    job.archived_len = 1;
    job.archived = vec![(0, item(&skin_png([0, 0, 200, 255]), true))];
    job.bundled = vec![(5, false)];
    let (generation, previews) = draw(job);
    assert_eq!(generation, 1, "the drawing answers with the job's generation");
    assert_eq!(previews.saved.len(), 3);
    assert!(previews.saved[0].is_some(), "a skin the viewer can draw has a preview");
    assert!(previews.saved[1].is_none(), "a place with no skin in the job has none");
    assert!(previews.saved[2].is_none(), "bytes that are not a skin have none");
    assert_eq!(previews.archived.len(), 1);
    assert!(previews.archived[0].is_some());
    assert_eq!(previews.bundled.len(), BUNDLED.len());
    assert!(previews.bundled[5].is_some(), "a bundled skin lands at its place in the catalog");
    assert_eq!(previews.bundled.iter().filter(|preview| preview.is_some()).count(), 1);
}

#[test]
fn previews_are_drawn_with_and_without_a_worn_cape() {
    let cape = {
        let image = image::RgbaImage::from_pixel(64, 32, image::Rgba([0, 255, 0, 255]));
        let mut bytes = std::io::Cursor::new(Vec::new());
        image.write_to(&mut bytes, image::ImageFormat::Png).expect("encodes");
        bytes.into_inner()
    };
    let mut caped = job(vec![(0, item(&skin_png([200, 0, 0, 255]), false))], 0.0);
    caped.cape = Some(cape);
    let (_, plain) = draw(job(vec![(0, item(&skin_png([200, 0, 0, 255]), false))], 0.0));
    let (_, worn) = draw(caped);
    assert!(plain.saved[0].is_some() && worn.saved[0].is_some());
}

#[test]
fn a_drawing_at_a_later_moment_of_the_pose_is_drawn_again() {
    // The same skin drawn at two moments of a walk gives two different pictures.
    let first = draw(job(vec![(0, item(&skin_png([200, 0, 0, 255]), false))], 0.0)).1;
    let later = draw(job(vec![(0, item(&skin_png([200, 0, 0, 255]), false))], 0.25)).1;
    assert!(first.saved[0].is_some() && later.saved[0].is_some());
}
