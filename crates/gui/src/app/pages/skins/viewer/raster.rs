//! A software rasteriser for the skin previews. It draws the triangles the viewer draws, through the same camera and sampling
//! the same texture with nearest filtering, into a pixel buffer on the CPU. A grid of skins can then show each one in 3D
//! without a GPU pipeline per tile, and the drawing runs off the UI thread.
//!
//! The triangles of a model depend on its pose, its moment, its layers, its arm width and its cape, but not on the skin's
//! texels. So a model is projected once, and every skin worn in it is filled from that one projection.

use super::geometry::model_vertices_with_cape;
use super::math::Mat4;
use super::{ATLAS_HEIGHT, ATLAS_WIDTH, Model, PITCH, YAW, view_proj};

/// The alpha below which a texel is see-through and draws nothing, as the viewer's shader discards it.
const CUTOFF: u8 = 128;

/// One corner of a triangle once the camera has projected it. Its attributes are divided by the clip-space `w`, so they can be
/// interpolated linearly across the triangle on the screen and divided back per pixel. That keeps the texture from swimming.
#[derive(Debug, Clone, Copy)]
struct Corner {
    x: f32,
    y: f32,
    inv_w: f32,
    depth_over_w: f32,
    u_over_w: f32,
    v_over_w: f32,
    shade: f32,
}

/// The triangles of a model on a `width` by `height` image, as the camera sees them. Every skin worn in the model is filled
/// from one projection.
pub struct Projection {
    width: u32,
    height: u32,
    triangles: Vec<[Corner; 3]>,
}

/// Projects `model` on a `width` by `height` image, with the cape when `cape` is set. The model's skin is not read, so one
/// projection serves every skin worn in the same model and pose.
pub fn project(model: &Model, cape: bool, width: u32, height: u32) -> Projection {
    let view = view_proj(width as f32 / height as f32, PITCH, YAW, model.pose);
    let triangles = model_vertices_with_cape(model, cape)
        .chunks_exact(3 * 6)
        .filter_map(|triangle| {
            let corner = |index: usize| corner(&view, width, height, &triangle[index * 6..index * 6 + 6]);
            Some([corner(0)?, corner(1)?, corner(2)?])
        })
        .collect();
    Projection { width, height, triangles }
}

/// Draws a projection with a skin's `texels`, on a clear background, as RGBA with straight alpha.
pub fn fill(projection: &Projection, texels: &[u8]) -> Vec<u8> {
    let (width, height) = (projection.width, projection.height);
    let mut pixels = vec![0u8; (width * height * 4) as usize];
    let mut depth = vec![f32::INFINITY; (width * height) as usize];
    for triangle in &projection.triangles {
        fill_triangle(&mut pixels, &mut depth, width, height, texels, *triangle);
    }
    pixels
}

/// `point` through `matrix`, to clip space as `[x, y, z, w]`.
fn clip(matrix: &Mat4, point: [f32; 3]) -> [f32; 4] {
    let mut out = [0.0; 4];
    for (row, value) in out.iter_mut().enumerate() {
        *value = matrix[0][row] * point[0] + matrix[1][row] * point[1] + matrix[2][row] * point[2] + matrix[3][row];
    }
    out
}

/// A vertex of the model, as the camera sees it on a `width` by `height` image. `None` when it is behind the camera.
fn corner(view: &Mat4, width: u32, height: u32, vertex: &[f32]) -> Option<Corner> {
    let [x, y, z, w] = clip(view, [vertex[0], vertex[1], vertex[2]]);
    if w <= 0.0 {
        return None;
    }
    let inv_w = 1.0 / w;
    let ndc_depth = z * inv_w;
    Some(Corner {
        x: (x * inv_w * 0.5 + 0.5) * width as f32,
        y: (0.5 - y * inv_w * 0.5) * height as f32,
        inv_w,
        depth_over_w: ndc_depth * inv_w,
        u_over_w: vertex[3] * inv_w,
        v_over_w: vertex[4] * inv_w,
        shade: vertex[5],
    })
}

/// A quantity that is linear in the screen position: `a * x + b * y + c`. Across a triangle, every attribute and every
/// barycentric weight is one, so a pixel's value is a few multiplies and no division.
#[derive(Debug, Clone, Copy)]
struct Plane {
    a: f32,
    b: f32,
    c: f32,
}

impl Plane {
    /// The value of the plane at the screen position `(x, y)`.
    fn at(self, x: f32, y: f32) -> f32 {
        self.a * x + self.b * y + self.c
    }

    /// The plane that sums each of `planes` scaled by its value in `values`.
    fn combine(planes: [Plane; 3], values: [f32; 3]) -> Plane {
        Plane {
            a: planes[0].a * values[0] + planes[1].a * values[1] + planes[2].a * values[2],
            b: planes[0].b * values[0] + planes[1].b * values[1] + planes[2].b * values[2],
            c: planes[0].c * values[0] + planes[1].c * values[1] + planes[2].c * values[2],
        }
    }
}

/// The barycentric weights of the corners `a`, `b` and `c`, each as a plane over the screen. Inside the triangle all three are
/// positive. `None` for a triangle with no area on the screen.
fn barycentric(a: Corner, b: Corner, c: Corner) -> Option<[Plane; 3]> {
    let det = (b.y - c.y) * (a.x - c.x) + (c.x - b.x) * (a.y - c.y);
    if det.abs() < f32::EPSILON {
        return None;
    }
    let inv = 1.0 / det;
    let weight_a = Plane {
        a: (b.y - c.y) * inv,
        b: (c.x - b.x) * inv,
        c: -(c.x * (b.y - c.y) + c.y * (c.x - b.x)) * inv,
    };
    let weight_b = Plane {
        a: (c.y - a.y) * inv,
        b: (a.x - c.x) * inv,
        c: -(c.x * (c.y - a.y) + c.y * (a.x - c.x)) * inv,
    };
    let weight_c = Plane {
        a: -(weight_a.a + weight_b.a),
        b: -(weight_a.b + weight_b.b),
        c: 1.0 - (weight_a.c + weight_b.c),
    };
    Some([weight_a, weight_b, weight_c])
}

/// The byte offset of the texel that the texture coordinates `(u, v)` sample, nearest, on the atlas.
fn texel_index(u: f32, v: f32) -> usize {
    let x = ((u * ATLAS_WIDTH as f32) as i64).clamp(0, ATLAS_WIDTH as i64 - 1) as usize;
    let y = ((v * ATLAS_HEIGHT as f32) as i64).clamp(0, ATLAS_HEIGHT as i64 - 1) as usize;
    (y * ATLAS_WIDTH as usize + x) * 4
}

/// Draws one triangle. Each pixel whose centre it covers takes the texel the triangle shows there, if nothing nearer has
/// been drawn there and the texel is not see-through.
fn fill_triangle(pixels: &mut [u8], depth: &mut [f32], width: u32, height: u32, texels: &[u8], [a, b, c]: [Corner; 3]) {
    let Some(weights) = barycentric(a, b, c) else {
        return;
    };
    // Each attribute is interpolated divided by the clip-space `w`, and divided back per pixel: perspective-correct.
    let inv_w = Plane::combine(weights, [a.inv_w, b.inv_w, c.inv_w]);
    let depth_over_w = Plane::combine(weights, [a.depth_over_w, b.depth_over_w, c.depth_over_w]);
    let u_over_w = Plane::combine(weights, [a.u_over_w, b.u_over_w, c.u_over_w]);
    let v_over_w = Plane::combine(weights, [a.v_over_w, b.v_over_w, c.v_over_w]);
    let shade = Plane::combine(weights, [a.shade, b.shade, c.shade]);
    let left = a.x.min(b.x).min(c.x).floor().max(0.0) as u32;
    let right = a.x.max(b.x).max(c.x).ceil().min(width as f32 - 1.0).max(0.0) as u32;
    let top = a.y.min(b.y).min(c.y).floor().max(0.0) as u32;
    let bottom = a.y.max(b.y).max(c.y).ceil().min(height as f32 - 1.0).max(0.0) as u32;
    for y in top..=bottom {
        for x in left..=right {
            let (px, py) = (x as f32 + 0.5, y as f32 + 0.5);
            let inside = weights[0].at(px, py) >= 0.0 && weights[1].at(px, py) >= 0.0 && weights[2].at(px, py) >= 0.0;
            if !inside {
                continue;
            }
            let index = (y * width + x) as usize;
            let w = inv_w.at(px, py);
            let depth_here = depth_over_w.at(px, py) / w;
            if depth_here >= depth[index] {
                continue;
            }
            let texel = texel_index(u_over_w.at(px, py) / w, v_over_w.at(px, py) / w);
            if texels[texel + 3] < CUTOFF {
                continue;
            }
            let light = shade.at(px, py);
            let out = index * 4;
            for channel in 0..3 {
                pixels[out + channel] = (f32::from(texels[texel + channel]) * light).round() as u8;
            }
            pixels[out + 3] = 255;
            depth[index] = depth_here;
        }
    }
}

#[cfg(test)]
#[path = "../../../../../tests/app/pages/skins/viewer/raster.rs"]
mod tests;
