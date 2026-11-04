//! The boxes of the model, and the cape, as the triangles the viewer draws.

use super::math::{Mat4, about, mul, rotate_x, rotate_z, transform_point, translate};
use super::{ATLAS_HEIGHT, ATLAS_WIDTH, CAPE_X, Model, Pose};

/// Where the cape's centre line leaves the shoulders: just behind the body's back face (z = -2) and the jacket layer (z = -2.25).
/// The cape is `CAPE_THICKNESS` deep, so its outside starts at z = -2.6, the depth it hung at before it had any thickness.
const CAPE_DEPTH: f32 = -2.35;
const CAPE_THICKNESS: f32 = 0.5;
const CAPE_TOP: f32 = 24.0;
const CAPE_LENGTH: f32 = 16.0;
const CAPE_HALF_WIDTH: f32 = 5.0;

/// The cape is cut into this many strips down its length, so that it can bend as it flows.
const CAPE_SEGMENTS: usize = 8;

/// The rows of the cape's art: its 16 texels down the sides, from the row below the top edge.
const CAPE_ART_HEIGHT: f32 = 16.0;

/// The shade of the cape's faces. The outside is the back of the model, which matches the back of a box. The inside faces the
/// body, and the edges and the lower edge match the sides and the bottom of a box.
const BACK_SHADE: f32 = 0.8;
const INSIDE_SHADE: f32 = 0.95;
const EDGE_SHADE: f32 = 0.85;
const LOWER_EDGE_SHADE: f32 = 0.6;

/// One box of the model: its corners in model pixels, its skin position, and the transform its pose gives it.
struct Part {
    origin: [f32; 3],
    size: [f32; 3],
    uv: (u32, u32),
    inflate: f32,
    transform: Mat4,
}

/// The seconds one stride of a walk takes, and of a run.
pub(super) const WALK_PERIOD: f32 = 1.0;
pub(super) const RUN_PERIOD: f32 = 0.6;

/// The seconds one sway of the standing, arm-raised and crouching poses takes.
pub(super) const STAND_PERIOD: f32 = 3.0;
pub(super) const ARMS_PERIOD: f32 = 1.2;
pub(super) const CROUCH_PERIOD: f32 = 2.0;

/// How far the body drops when the model crouches, in model pixels.
const CROUCH_DROP: f32 = 3.0;

/// Where the limbs are, and how far the body is lifted, at one moment of a pose: the arms as `(z, x)` turns, the legs' swing,
/// and the lift in model pixels.
#[derive(Debug, Clone, Copy, PartialEq)]
struct Motion {
    right_arm: (f32, f32),
    left_arm: (f32, f32),
    right_leg: f32,
    left_leg: f32,
    lift: f32,
}

/// The motion of `pose` `time` seconds in. A walk or a run swings the arms against the legs through its stride and lifts the
/// body once per step. The other poses keep their shape and sway gently.
fn motion(pose: Pose, time: f32) -> Motion {
    use std::f32::consts::{FRAC_PI_2, TAU};
    match pose {
        Pose::Stand => {
            let sway = 0.04 * (TAU * time / STAND_PERIOD).sin();
            Motion {
                right_arm: (0.0, sway),
                left_arm: (0.0, -sway),
                right_leg: 0.0,
                left_leg: 0.0,
                lift: 0.0,
            }
        }
        Pose::Arms => {
            // The arms stay out to the sides, and flap slowly.
            let flap = 0.12 * (TAU * time / ARMS_PERIOD).sin();
            Motion {
                right_arm: (-FRAC_PI_2 + flap, 0.0),
                left_arm: (FRAC_PI_2 - flap, 0.0),
                right_leg: 0.0,
                left_leg: 0.0,
                lift: 0.0,
            }
        }
        Pose::Walk => stride(time, WALK_PERIOD, 0.5, 0.06),
        Pose::Run => stride(time, RUN_PERIOD, 0.9, 0.12),
        Pose::Crouch => {
            let sway = 0.05 * (TAU * time / CROUCH_PERIOD).sin();
            Motion {
                right_arm: (0.0, -0.6 + sway),
                left_arm: (0.0, -0.6 - sway),
                right_leg: -0.6,
                left_leg: -0.6,
                lift: 0.0,
            }
        }
    }
}

/// The swing of a stride: the legs swing with the period, the arms against them, and the body lifts twice per step, by `bob`
/// at most.
fn stride(time: f32, period: f32, swing: f32, bob: f32) -> Motion {
    let phase = std::f32::consts::TAU * time / period;
    let arm = swing * phase.sin();
    Motion {
        right_arm: (0.0, arm),
        left_arm: (0.0, -arm),
        right_leg: -arm,
        left_leg: arm,
        lift: bob * (1.0 - (2.0 * phase).cos()) / 2.0,
    }
}

/// The transform of the body. It drops when the model crouches and lifts with the stride, and the legs, arms and cape move
/// with it.
fn body_transform(model: &Model) -> Mat4 {
    let crouch = if model.pose == Pose::Crouch { CROUCH_DROP } else { 0.0 };
    let lift = motion(model.pose, model.time).lift;
    translate(0.0, -crouch - lift, 0.0)
}

/// The boxes that make up the model for a model's pose, layers and arm width.
fn parts(model: &Model) -> Vec<Part> {
    let (arm_width, arm_x) = if model.slim { (3.0, 1.0) } else { (4.0, 0.0) };
    let Motion {
        right_arm,
        left_arm,
        right_leg,
        left_leg,
        ..
    } = motion(model.pose, model.time);
    let body_transform = body_transform(model);
    let mut out = vec![
        Part {
            origin: [-4.0, 24.0, -4.0],
            size: [8.0, 8.0, 8.0],
            uv: (0, 0),
            inflate: 0.0,
            transform: body_transform,
        },
        Part {
            origin: [-4.0, 12.0, -2.0],
            size: [8.0, 12.0, 4.0],
            uv: (16, 16),
            inflate: 0.0,
            transform: body_transform,
        },
        Part {
            origin: [-8.0 + arm_x, 12.0, -2.0],
            size: [arm_width, 12.0, 4.0],
            uv: (40, 16),
            inflate: 0.0,
            transform: mul(
                body_transform,
                about([-6.0, 22.0, 0.0], mul(rotate_z(right_arm.0), rotate_x(right_arm.1))),
            ),
        },
        Part {
            origin: [4.0, 12.0, -2.0],
            size: [arm_width, 12.0, 4.0],
            uv: (32, 48),
            inflate: 0.0,
            transform: mul(body_transform, about([6.0, 22.0, 0.0], mul(rotate_z(left_arm.0), rotate_x(left_arm.1)))),
        },
        Part {
            origin: [-4.0, 0.0, -2.0],
            size: [4.0, 12.0, 4.0],
            uv: (0, 16),
            inflate: 0.0,
            transform: mul(body_transform, about([-2.0, 12.0, 0.0], rotate_x(right_leg))),
        },
        Part {
            origin: [0.0, 0.0, -2.0],
            size: [4.0, 12.0, 4.0],
            uv: (16, 48),
            inflate: 0.0,
            transform: mul(body_transform, about([2.0, 12.0, 0.0], rotate_x(left_leg))),
        },
    ];
    if model.layers.hat {
        out.push(Part {
            origin: [-4.0, 24.0, -4.0],
            size: [8.0, 8.0, 8.0],
            uv: (32, 0),
            inflate: 0.5,
            transform: body_transform,
        });
    }
    if model.layers.jacket {
        out.push(Part {
            origin: [-4.0, 12.0, -2.0],
            size: [8.0, 12.0, 4.0],
            uv: (16, 32),
            inflate: 0.25,
            transform: body_transform,
        });
    }
    if model.layers.sleeves {
        out.push(Part {
            origin: [-8.0 + arm_x, 12.0, -2.0],
            size: [arm_width, 12.0, 4.0],
            uv: (40, 32),
            inflate: 0.25,
            transform: out[2].transform,
        });
        out.push(Part {
            origin: [4.0, 12.0, -2.0],
            size: [arm_width, 12.0, 4.0],
            uv: (48, 48),
            inflate: 0.25,
            transform: out[3].transform,
        });
    }
    if model.layers.pants {
        out.push(Part {
            origin: [-4.0, 0.0, -2.0],
            size: [4.0, 12.0, 4.0],
            uv: (0, 32),
            inflate: 0.25,
            transform: out[4].transform,
        });
        out.push(Part {
            origin: [0.0, 0.0, -2.0],
            size: [4.0, 12.0, 4.0],
            uv: (0, 48),
            inflate: 0.25,
            transform: out[5].transform,
        });
    }
    out
}

/// The vertices of the model: six faces per box, each as two triangles, with the texel coordinates and the face's shade.
/// The cape follows the boxes when the skin carries a cape.
pub(super) fn model_vertices(model: &Model) -> Vec<f32> {
    model_vertices_with_cape(model, model.skin.as_ref().is_some_and(|skin| skin.cape))
}

/// The vertices of the model, with the cape when `cape` is set. A projection of the model carries no skin, so it passes the
/// cape itself.
pub(super) fn model_vertices_with_cape(model: &Model, cape: bool) -> Vec<f32> {
    let mut vertices = Vec::new();
    for part in parts(model) {
        push_box(&mut vertices, &part);
    }
    if cape {
        push_cape(&mut vertices, body_transform(model), model.pose, model.time);
    }
    vertices
}

/// One face of a box: its four corners in order, the texel rectangle `(u, v, width, height)` it takes from the skin, and its
/// shade.
type Face = ([[f32; 3]; 4], (f32, f32, f32, f32), f32);

/// The faces of one box. Each face is the rectangle of the skin that paints it, and each corner of the rectangle lands on
/// a corner of the box.
fn push_box(vertices: &mut Vec<f32>, part: &Part) {
    let (u, v) = (part.uv.0 as f32, part.uv.1 as f32);
    let (w, h, d) = (part.size[0], part.size[1], part.size[2]);
    let (wu, hv, du) = (w, h, d);
    let (min, max) = (
        [
            part.origin[0] - part.inflate,
            part.origin[1] - part.inflate,
            part.origin[2] - part.inflate,
        ],
        [
            part.origin[0] + w + part.inflate,
            part.origin[1] + h + part.inflate,
            part.origin[2] + d + part.inflate,
        ],
    );
    let (x0, y0, z0) = (min[0], min[1], min[2]);
    let (x1, y1, z1) = (max[0], max[1], max[2]);
    // Each face: four corners in order, the texel rectangle (u, v, width, height), and its shade.
    let faces: [Face; 6] = [
        // front (+z)
        ([[x0, y0, z1], [x1, y0, z1], [x1, y1, z1], [x0, y1, z1]], (u + du, v + du, wu, hv), 0.95),
        // back (-z)
        (
            [[x1, y0, z0], [x0, y0, z0], [x0, y1, z0], [x1, y1, z0]],
            (u + 2.0 * du + wu, v + du, wu, hv),
            0.8,
        ),
        // right side (+x)
        (
            [[x1, y0, z1], [x1, y0, z0], [x1, y1, z0], [x1, y1, z1]],
            (u + du + wu, v + du, du, hv),
            0.85,
        ),
        // left side (-x)
        ([[x0, y0, z0], [x0, y0, z1], [x0, y1, z1], [x0, y1, z0]], (u, v + du, du, hv), 0.85),
        // top (+y)
        ([[x0, y1, z1], [x1, y1, z1], [x1, y1, z0], [x0, y1, z0]], (u + du, v, wu, du), 1.0),
        // bottom (-y)
        ([[x0, y0, z0], [x1, y0, z0], [x1, y0, z1], [x0, y0, z1]], (u + du + wu, v, wu, du), 0.6),
    ];
    for (corners, (texel_u, texel_v, texel_w, texel_h), shade) in faces {
        let uv = [
            [texel_u, texel_v + texel_h],
            [texel_u + texel_w, texel_v + texel_h],
            [texel_u + texel_w, texel_v],
            [texel_u, texel_v],
        ];
        let vertex = |index: usize| {
            let p = transform_point(part.transform, corners[index]);
            [
                p[0],
                p[1],
                p[2],
                uv[index][0] / ATLAS_WIDTH as f32,
                uv[index][1] / ATLAS_HEIGHT as f32,
                shade,
            ]
        };
        for index in [0, 1, 2, 0, 2, 3] {
            vertices.extend_from_slice(&vertex(index));
        }
    }
}

/// How far a pose swings the cape back, in radians at its lower edge, and how far a ripple runs down it. A walk and a run
/// stream the cape out behind the body further than a stand does.
fn cape_flow(pose: Pose) -> (f32, f32) {
    match pose {
        Pose::Stand => (0.45, 0.06),
        Pose::Arms => (0.5, 0.1),
        Pose::Walk => (0.6, 0.14),
        Pose::Run => (0.8, 0.22),
        Pose::Crouch => (0.4, 0.04),
    }
}

/// The angle the cape leans back at `fraction` of its length down from the shoulders, `time` seconds into the pose. The lean
/// grows down the cape, and a ripple travels down it once per loop of the pose, so the lower edge flows behind the body.
fn cape_angle(pose: Pose, fraction: f32, time: f32) -> f32 {
    use std::f32::consts::{PI, TAU};
    let (lean, ripple) = cape_flow(pose);
    let phase = TAU * time / pose.loop_seconds() - PI * fraction;
    fraction * (lean + ripple * phase.sin())
}

/// A cross-section of the cape: where its centre line is at one strip's end, in the body's height and depth, and the
/// direction that points out of its back, as `(y, z)`.
struct Station {
    fraction: f32,
    y: f32,
    z: f32,
    normal: [f32; 2],
}

/// The cross-sections down the cape, from the shoulders to the lower edge. Each strip carries the centre line on along the
/// angle at its middle, so the cape curves rather than folding at the seams.
fn cape_stations(pose: Pose, time: f32) -> Vec<Station> {
    let segment = CAPE_LENGTH / CAPE_SEGMENTS as f32;
    let (mut y, mut z) = (CAPE_TOP, CAPE_DEPTH);
    let mut stations = Vec::with_capacity(CAPE_SEGMENTS + 1);
    for index in 0..=CAPE_SEGMENTS {
        let fraction = index as f32 / CAPE_SEGMENTS as f32;
        let angle = cape_angle(pose, fraction, time);
        stations.push(Station {
            fraction,
            y,
            z,
            normal: [angle.sin(), -angle.cos()],
        });
        let middle = cape_angle(pose, fraction + 0.5 / CAPE_SEGMENTS as f32, time);
        y -= segment * middle.cos();
        z -= segment * middle.sin();
    }
    stations
}

/// The point across the cape at `station`, at `x`, on its outside (`side` of 1) or its inside (`side` of -1).
fn cape_point(station: &Station, x: f32, side: f32) -> [f32; 3] {
    let half = side * CAPE_THICKNESS / 2.0;
    [x, station.y + half * station.normal[0], station.z + half * station.normal[1]]
}

/// One quad of the cape: its four corners in order, the cape's texel of each corner, and its shade. The texels are on the
/// cape's own 64×32 block, which starts at `CAPE_X` on the texture.
fn push_cape_quad(vertices: &mut Vec<f32>, transform: Mat4, corners: [[f32; 3]; 4], texels: [(f32, f32); 4], shade: f32) {
    for index in [0, 1, 2, 0, 2, 3] {
        let p = transform_point(transform, corners[index]);
        let (u, v) = texels[index];
        vertices.extend_from_slice(&[p[0], p[1], p[2], (CAPE_X as f32 + u) / ATLAS_WIDTH as f32, v / ATLAS_HEIGHT as f32, shade]);
    }
}

/// The cape: a thin sheet hung from the shoulders, drawn in the cape texture's own layout. Its outside is the 10×16 block at
/// (1, 1), its inside the block beside it at (12, 1), and its edges and lower edge are the thin strips around them. The sheet
/// bends back as it hangs, so it stands out behind the body, and it moves with the body.
fn push_cape(vertices: &mut Vec<f32>, transform: Mat4, pose: Pose, time: f32) {
    let stations = cape_stations(pose, time);
    let half = CAPE_HALF_WIDTH;
    // The texel row at a station's place down the cape. The art's top row is row 1, below the cape's top edge.
    let row = |station: &Station| 1.0 + CAPE_ART_HEIGHT * station.fraction;
    for strip in stations.windows(2) {
        let (top, bottom) = (&strip[0], &strip[1]);
        let (top_v, bottom_v) = (row(top), row(bottom));
        // As seen from behind, the player's left is on the viewer's left, which is the model's +x side. The outside reads
        // that way, and the inside reads it mirrored, as a box's faces do.
        push_cape_quad(
            vertices,
            transform,
            [
                cape_point(top, half, 1.0),
                cape_point(top, -half, 1.0),
                cape_point(bottom, -half, 1.0),
                cape_point(bottom, half, 1.0),
            ],
            [(1.0, top_v), (11.0, top_v), (11.0, bottom_v), (1.0, bottom_v)],
            BACK_SHADE,
        );
        push_cape_quad(
            vertices,
            transform,
            [
                cape_point(top, half, -1.0),
                cape_point(top, -half, -1.0),
                cape_point(bottom, -half, -1.0),
                cape_point(bottom, half, -1.0),
            ],
            [(22.0, top_v), (12.0, top_v), (12.0, bottom_v), (22.0, bottom_v)],
            INSIDE_SHADE,
        );
        push_cape_quad(
            vertices,
            transform,
            [
                cape_point(top, half, 1.0),
                cape_point(top, half, -1.0),
                cape_point(bottom, half, -1.0),
                cape_point(bottom, half, 1.0),
            ],
            [(11.0, top_v), (12.0, top_v), (12.0, bottom_v), (11.0, bottom_v)],
            EDGE_SHADE,
        );
        push_cape_quad(
            vertices,
            transform,
            [
                cape_point(top, -half, -1.0),
                cape_point(top, -half, 1.0),
                cape_point(bottom, -half, 1.0),
                cape_point(bottom, -half, -1.0),
            ],
            [(1.0, top_v), (0.0, top_v), (0.0, bottom_v), (1.0, bottom_v)],
            EDGE_SHADE,
        );
    }
    if let Some(lower) = stations.last() {
        push_cape_quad(
            vertices,
            transform,
            [
                cape_point(lower, half, 1.0),
                cape_point(lower, -half, 1.0),
                cape_point(lower, -half, -1.0),
                cape_point(lower, half, -1.0),
            ],
            [(11.0, 0.0), (21.0, 0.0), (21.0, 1.0), (11.0, 1.0)],
            LOWER_EDGE_SHADE,
        );
    }
}

#[cfg(test)]
#[path = "../../../../../tests/app/pages/skins/viewer/geometry.rs"]
mod tests;
