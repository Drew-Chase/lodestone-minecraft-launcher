use std::sync::Arc;

use super::super::{Layers, Pose, Skin};
use super::*;

/// A skin whose every texel is `color`, with no cape.
fn skin(color: [u8; 4]) -> Skin {
    let texels: Vec<u8> = (0..ATLAS_WIDTH * ATLAS_HEIGHT).flat_map(|_| color).collect();
    Skin { id: 0, texels, cape: false }
}

/// A model wearing `skin` in `pose`, at the moment the pose is held still.
fn model(skin: Skin, pose: Pose) -> Model {
    Model {
        skin: Some(Arc::new(skin)),
        pose,
        layers: Layers::default(),
        slim: false,
        time: pose.preview_time(),
    }
}

/// `model` drawn the way a preview draws it: projected once, then filled with its skin's texels. A model with no skin draws
/// nothing, over a clear background.
fn render(model: &Model, width: u32, height: u32) -> Vec<u8> {
    match model.skin.as_ref() {
        Some(skin) => fill(&project(model, skin.cape, width, height), &skin.texels),
        None => vec![0u8; (width * height * 4) as usize],
    }
}

/// The RGBA of the pixel at `(x, y)` of a `width`-wide image.
fn pixel(pixels: &[u8], width: u32, x: u32, y: u32) -> [u8; 4] {
    let at = ((y * width + x) * 4) as usize;
    [pixels[at], pixels[at + 1], pixels[at + 2], pixels[at + 3]]
}

#[test]
fn the_model_is_drawn_over_a_clear_background() {
    let pixels = render(&model(skin([200, 40, 40, 255]), Pose::Stand), 96, 128);
    assert_eq!(pixel(&pixels, 96, 0, 0)[3], 0, "the corner is clear");
    let centre = pixel(&pixels, 96, 48, 64);
    assert_eq!(centre[3], 255, "the middle of the body is drawn");
    assert!(centre[0] > centre[1] && centre[0] > centre[2], "the texel's red shows: {centre:?}");
}

#[test]
fn the_model_fills_the_height_and_stays_inside_it() {
    let pixels = render(&model(skin([200, 40, 40, 255]), Pose::Stand), 96, 128);
    let row_drawn = |y: u32| (0..96).any(|x| pixel(&pixels, 96, x, y)[3] > 0);
    assert!(!row_drawn(0), "nothing draws on the top row");
    assert!(!row_drawn(127), "nothing draws on the bottom row");
    let drawn_rows = (0..128).filter(|&y| row_drawn(y)).count();
    assert!(drawn_rows >= 100, "the model fills most of the height, not {drawn_rows} rows");
}

#[test]
fn the_arms_of_a_wide_pose_keep_clear_of_the_sides() {
    let pixels = render(&model(skin([200, 40, 40, 255]), Pose::Arms), 96, 128);
    let column_drawn = |x: u32| (0..128).any(|y| pixel(&pixels, 96, x, y)[3] > 0);
    assert!(!column_drawn(0), "the left edge is clear");
    assert!(!column_drawn(95), "the right edge is clear");
}

#[test]
fn a_see_through_skin_draws_nothing() {
    let pixels = render(&model(skin([200, 40, 40, 0]), Pose::Stand), 96, 128);
    assert!(pixels.chunks_exact(4).all(|pixel| pixel[3] == 0));
}

#[test]
fn a_model_without_a_skin_draws_nothing() {
    let nothing = Model {
        skin: None,
        pose: Pose::Stand,
        layers: Layers::default(),
        slim: false,
        time: 0.0,
    };
    assert!(render(&nothing, 96, 128).iter().all(|&byte| byte == 0));
}

#[test]
fn a_projection_filled_with_a_skin_is_the_render_of_that_skin() {
    let model = model(skin([200, 40, 40, 255]), Pose::Stand);
    let texels = model.skin.as_ref().expect("the model has a skin").texels.clone();
    assert!(fill(&project(&model, false, 96, 128), &texels) == render(&model, 96, 128));
}

#[test]
fn one_projection_draws_each_skin_worn_in_it() {
    let red = model(skin([200, 40, 40, 255]), Pose::Stand);
    let blue_texels = model(skin([40, 40, 200, 255]), Pose::Stand).skin.expect("a skin").texels.clone();
    let drawn = fill(&project(&red, false, 96, 128), &blue_texels);
    let centre = pixel(&drawn, 96, 48, 64);
    assert!(
        centre[2] > centre[0] && centre[2] > centre[1],
        "the blue skin shows through the red model's projection: {centre:?}"
    );
}
