//! The matrices the viewer's camera, limbs and cape are built from.

/// A column-major 4×4 matrix, as the shader reads it.
pub(super) type Mat4 = [[f32; 4]; 4];

/// The identity matrix.
fn identity() -> Mat4 {
    [[1.0, 0.0, 0.0, 0.0], [0.0, 1.0, 0.0, 0.0], [0.0, 0.0, 1.0, 0.0], [0.0, 0.0, 0.0, 1.0]]
}

/// `a` times `b`: applying `b` first, then `a`.
pub(super) fn mul(a: Mat4, b: Mat4) -> Mat4 {
    let mut result = [[0.0; 4]; 4];
    for (column, out) in result.iter_mut().enumerate() {
        for (row, value) in out.iter_mut().enumerate() {
            *value = (0..4).map(|k| a[k][row] * b[column][k]).sum();
        }
    }
    result
}

/// A translation by `(x, y, z)`.
pub(super) fn translate(x: f32, y: f32, z: f32) -> Mat4 {
    let mut matrix = identity();
    matrix[3] = [x, y, z, 1.0];
    matrix
}

/// A uniform scale.
pub(super) fn scale(factor: f32) -> Mat4 {
    let mut matrix = identity();
    matrix[0][0] = factor;
    matrix[1][1] = factor;
    matrix[2][2] = factor;
    matrix
}

/// A rotation about the x axis.
pub(super) fn rotate_x(angle: f32) -> Mat4 {
    let (sine, cosine) = angle.sin_cos();
    [
        [1.0, 0.0, 0.0, 0.0],
        [0.0, cosine, sine, 0.0],
        [0.0, -sine, cosine, 0.0],
        [0.0, 0.0, 0.0, 1.0],
    ]
}

/// A rotation about the y axis.
pub(super) fn rotate_y(angle: f32) -> Mat4 {
    let (sine, cosine) = angle.sin_cos();
    [
        [cosine, 0.0, -sine, 0.0],
        [0.0, 1.0, 0.0, 0.0],
        [sine, 0.0, cosine, 0.0],
        [0.0, 0.0, 0.0, 1.0],
    ]
}

/// A rotation about the z axis.
pub(super) fn rotate_z(angle: f32) -> Mat4 {
    let (sine, cosine) = angle.sin_cos();
    [
        [cosine, sine, 0.0, 0.0],
        [-sine, cosine, 0.0, 0.0],
        [0.0, 0.0, 1.0, 0.0],
        [0.0, 0.0, 0.0, 1.0],
    ]
}

/// A perspective projection onto the 0 to 1 depth range that the GPU uses.
pub(super) fn perspective(fov_y: f32, aspect: f32, near: f32, far: f32) -> Mat4 {
    let focal = 1.0 / (fov_y / 2.0).tan();
    [
        [focal / aspect, 0.0, 0.0, 0.0],
        [0.0, focal, 0.0, 0.0],
        [0.0, 0.0, far / (near - far), -1.0],
        [0.0, 0.0, far * near / (near - far), 0.0],
    ]
}

/// The matrix flattened in column order, for the uniform buffer.
pub(super) fn flatten(matrix: Mat4) -> Vec<f32> {
    matrix.iter().flatten().copied().collect()
}

/// Floats as the bytes the GPU reads.
pub(super) fn floats_to_bytes(floats: &[f32]) -> Vec<u8> {
    floats.iter().flat_map(|float| float.to_ne_bytes()).collect()
}

/// A rotation about `pivot`: the transform of a limb that swings from its joint.
pub(super) fn about(pivot: [f32; 3], rotation: Mat4) -> Mat4 {
    mul(
        translate(pivot[0], pivot[1], pivot[2]),
        mul(rotation, translate(-pivot[0], -pivot[1], -pivot[2])),
    )
}

/// `point` moved by `matrix`.
pub(super) fn transform_point(matrix: Mat4, point: [f32; 3]) -> [f32; 3] {
    let mut out = [0.0; 3];
    for (row, value) in out.iter_mut().enumerate() {
        *value = matrix[0][row] * point[0] + matrix[1][row] * point[1] + matrix[2][row] * point[2] + matrix[3][row];
    }
    out
}
