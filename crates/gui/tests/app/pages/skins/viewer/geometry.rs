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

#[test]
fn a_worn_cape_adds_one_quad_behind_the_body() {
    let plain = model_vertices(&model(false));
    let caped = model_vertices(&model(true));
    assert_eq!(caped.len(), plain.len() + 6 * 6, "one quad is six vertices");
    for vertex in caped[plain.len()..].chunks_exact(6) {
        assert!((vertex[2] - CAPE_DEPTH).abs() < 1e-5, "the cape sits behind the body");
        assert!(vertex[3] >= CAPE_X as f32 / ATLAS_WIDTH as f32, "the cape is on the texture's right half");
        assert!(vertex[4] <= 17.0 / ATLAS_HEIGHT as f32, "the cape's art stops at its own 17th texel");
    }
}

#[test]
fn the_cape_hangs_from_the_shoulders_to_the_top_of_the_legs() {
    let caped = model_vertices(&model(true));
    let quad = &caped[caped.len() - 36..];
    let top = quad.chunks_exact(6).map(|vertex| vertex[1]).fold(f32::MIN, f32::max);
    let bottom = quad.chunks_exact(6).map(|vertex| vertex[1]).fold(f32::MAX, f32::min);
    assert_eq!(top, 24.0, "the shoulders");
    assert_eq!(bottom, 8.0, "the top of the legs");
}

#[test]
fn crouching_lowers_the_cape_with_the_body() {
    let mut crouched = model(true);
    crouched.pose = Pose::Crouch;
    let vertices = model_vertices(&crouched);
    let quad = &vertices[vertices.len() - 36..];
    let top = quad.chunks_exact(6).map(|vertex| vertex[1]).fold(f32::MIN, f32::max);
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
