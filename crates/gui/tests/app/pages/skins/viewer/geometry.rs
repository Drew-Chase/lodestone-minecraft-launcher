use std::sync::Arc;

use super::super::{Layers, Skin};
use super::*;

/// The floats of one box: six faces, two triangles of three vertices each, and six floats a vertex.
const BOX_FLOATS: usize = 6 * 2 * 3 * 6;

/// A model with every layer on, in the standing pose at the start of its motion, with or without a cape.
fn model(cape: bool) -> Model {
    Model {
        skin: Some(Arc::new(Skin {
            id: 0,
            texels: Vec::new(),
            cape,
        })),
        pose: Pose::Stand,
        layers: Layers::default(),
        slim: false,
        time: 0.0,
    }
}

#[test]
fn every_layer_is_twelve_boxes() {
    // The head, body, two arms and two legs, and the six overlay boxes: the hat, the jacket, two sleeves and two pants.
    assert_eq!(model_vertices(&model(false)).len(), 12 * BOX_FLOATS);
}

/// The floats of the cape: its outside, inside, two edges and lower edge, each strip a quad of six vertices.
const CAPE_FLOATS: usize = (4 * CAPE_SEGMENTS + 1) * 6 * 6;

/// The cape's vertices, which the model's are followed by.
fn cape_of(vertices: &[f32]) -> &[f32] {
    &vertices[vertices.len() - CAPE_FLOATS..]
}

fn vertices_of(vertices: &[f32]) -> impl Iterator<Item = &[f32]> {
    vertices.chunks_exact(6)
}

#[test]
fn a_worn_cape_adds_a_sheet_behind_the_body() {
    let plain = model_vertices(&model(false));
    let caped = model_vertices(&model(true));
    assert_eq!(caped.len(), plain.len() + CAPE_FLOATS, "the cape is a strip of quads down its length");
    for vertex in vertices_of(cape_of(&caped)) {
        assert!(vertex[2] < -2.0, "the cape sits behind the body's back face");
        assert!(vertex[3] >= CAPE_X as f32 / ATLAS_WIDTH as f32, "the cape is on the texture's right half");
        assert!(vertex[4] <= 17.0 / ATLAS_HEIGHT as f32, "the cape's art stops at its own 17th texel");
    }
}

#[test]
fn the_cape_is_three_dimensional() {
    let caped = model_vertices(&model(true));
    let shades: Vec<f32> = vertices_of(cape_of(&caped)).map(|vertex| vertex[5]).collect();
    assert!(shades.contains(&BACK_SHADE), "the outside is drawn");
    assert!(shades.contains(&INSIDE_SHADE), "the inside is drawn");
    assert!(shades.contains(&EDGE_SHADE), "the edges are drawn");
}

#[test]
fn the_cape_hangs_from_the_shoulders() {
    let caped = model_vertices(&model(true));
    let cape = cape_of(&caped);
    let top = vertices_of(cape).map(|vertex| vertex[1]).fold(f32::MIN, f32::max);
    let bottom = vertices_of(cape).map(|vertex| vertex[1]).fold(f32::MAX, f32::min);
    assert_eq!(top, 24.0, "the shoulders");
    assert!(
        bottom > 8.0 && bottom < 12.0,
        "the lower edge is a little above the top of the legs, as the cape leans back: {bottom}"
    );
}

#[test]
fn the_cape_flows_out_behind_the_body() {
    let caped = model_vertices(&model(true));
    let deepest = vertices_of(cape_of(&caped)).map(|vertex| vertex[2]).fold(f32::MAX, f32::min);
    assert!(deepest < -4.0, "the lower edge stands well behind the body: {deepest}");
}

#[test]
fn a_run_streams_the_cape_further_back_than_a_stand() {
    let deepest = |pose: Pose| {
        let mut posed = model(true);
        posed.pose = pose;
        let vertices = model_vertices(&posed);
        vertices_of(cape_of(&vertices)).map(|vertex| vertex[2]).fold(f32::MAX, f32::min)
    };
    assert!(deepest(Pose::Run) < deepest(Pose::Stand) - 1.0);
}

#[test]
fn crouching_lowers_the_cape_with_the_body() {
    let mut crouched = model(true);
    crouched.pose = Pose::Crouch;
    let vertices = model_vertices(&crouched);
    let top = vertices_of(cape_of(&vertices)).map(|vertex| vertex[1]).fold(f32::MIN, f32::max);
    assert_eq!(top, 21.0);
}

#[test]
fn a_walk_held_at_its_preview_time_is_its_full_stride() {
    let stride = motion(Pose::Walk, Pose::Walk.preview_time());
    assert!((stride.right_arm.1 - 0.5).abs() < 1e-5, "the right arm swings forward");
    assert!((stride.right_leg + 0.5).abs() < 1e-5, "the right leg swings back");
    assert!((stride.left_leg - 0.5).abs() < 1e-5, "the left leg swings forward");
}

#[test]
fn a_walk_swings_each_leg_against_the_arm_on_its_side() {
    let start = motion(Pose::Walk, 0.0);
    assert_eq!(start.right_leg, 0.0, "a walk starts with the limbs under the body");
    let later = motion(Pose::Walk, WALK_PERIOD * 0.75);
    assert!(later.right_leg > 0.4, "the right leg has swung forward by three quarters of a stride");
    assert!(later.right_arm.1 < -0.4, "the right arm swings back as the right leg comes forward");
}

#[test]
fn a_run_lifts_the_body_once_per_step_and_never_past_its_bob() {
    for step in 0..100 {
        let time = RUN_PERIOD * step as f32 / 100.0;
        let lift = motion(Pose::Run, time).lift;
        assert!((0.0..=0.12 + 1e-5).contains(&lift), "the lift at {time} is {lift}");
    }
}

#[test]
fn the_stand_pose_sways_within_a_small_angle() {
    for step in 0..60 {
        let arm = motion(Pose::Stand, step as f32 * 0.1).right_arm.1;
        assert!(arm.abs() <= 0.04 + 1e-6, "the arm sways {arm} at step {step}");
    }
}

#[test]
fn a_lifted_body_takes_the_legs_and_the_cape_with_it() {
    let mut running = model(true);
    running.pose = Pose::Run;
    running.time = RUN_PERIOD / 4.0;
    let lift = motion(Pose::Run, running.time).lift;
    assert!(lift > 0.0, "the stride lifts the body at this moment");
    let body = body_transform(&running);
    assert!(
        (body[3][1] + lift).abs() < 1e-6,
        "the body is shifted down by the lift, which the legs share"
    );
}
