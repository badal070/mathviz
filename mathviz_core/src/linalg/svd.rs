use nalgebra::{DMatrix, Matrix2, Matrix3, Vector2, Vector3};

use crate::{
    error::{MathvizError, MathvizResult},
    types::{AxisSpec, GeometryBuffer},
};

pub fn svd_layers(
    matrix: &[Vec<f64>],
    x_axis: &AxisSpec,
    y_axis: &AxisSpec,
    z_axis: Option<&AxisSpec>,
    layer_prefix: &str,
) -> MathvizResult<Vec<GeometryBuffer>> {
    if matrix.len() == 2 && matrix.iter().all(|r| r.len() == 2) {
        svd_2d(matrix, x_axis, y_axis, layer_prefix)
    } else if matrix.len() == 3 && matrix.iter().all(|r| r.len() == 3) {
        let z = z_axis.ok_or_else(|| {
            MathvizError::DomainViolation("3x3 SVD requires z axis domain".to_string())
        })?;
        svd_3d(matrix, x_axis, y_axis, z, layer_prefix)
    } else {
        Err(MathvizError::DomainViolation(
            "matrix must be 2x2 or 3x3 for SVD".to_string(),
        ))
    }
}

fn svd_2d(
    matrix: &[Vec<f64>],
    x: &AxisSpec,
    y: &AxisSpec,
    prefix: &str,
) -> MathvizResult<Vec<GeometryBuffer>> {
    let m = DMatrix::from_row_slice(
        2,
        2,
        &[matrix[0][0], matrix[0][1], matrix[1][0], matrix[1][1]],
    );
    let svd = m.svd(true, true);
    let u = svd
        .u
        .ok_or_else(|| MathvizError::EvalError("SVD U missing".to_string()))?;
    let vt = svd
        .v_t
        .ok_or_else(|| MathvizError::EvalError("SVD Vt missing".to_string()))?;
    let s = svd.singular_values;

    let v_t = Matrix2::new(vt[(0, 0)], vt[(0, 1)], vt[(1, 0)], vt[(1, 1)]);
    let sigma = Matrix2::new(s[0], 0.0, 0.0, s[1]);
    let u2 = Matrix2::new(u[(0, 0)], u[(0, 1)], u[(1, 0)], u[(1, 1)]);

    let lattice = lattice_2d_lines(x, y, 12);
    let layer1 = apply_2d(&lattice, v_t, format!("{prefix}_svd_vt"));
    let layer2 = apply_2d(&lattice, sigma * v_t, format!("{prefix}_svd_sigma"));
    let layer3 = apply_2d(&lattice, u2 * sigma * v_t, format!("{prefix}_svd_u"));

    Ok(vec![layer1, layer2, layer3])
}

fn svd_3d(
    matrix: &[Vec<f64>],
    x: &AxisSpec,
    y: &AxisSpec,
    z: &AxisSpec,
    prefix: &str,
) -> MathvizResult<Vec<GeometryBuffer>> {
    let m = DMatrix::from_row_slice(
        3,
        3,
        &[
            matrix[0][0],
            matrix[0][1],
            matrix[0][2],
            matrix[1][0],
            matrix[1][1],
            matrix[1][2],
            matrix[2][0],
            matrix[2][1],
            matrix[2][2],
        ],
    );
    let svd = m.svd(true, true);
    let u = svd
        .u
        .ok_or_else(|| MathvizError::EvalError("SVD U missing".to_string()))?;
    let vt = svd
        .v_t
        .ok_or_else(|| MathvizError::EvalError("SVD Vt missing".to_string()))?;
    let s = svd.singular_values;

    let v_t = Matrix3::new(
        vt[(0, 0)],
        vt[(0, 1)],
        vt[(0, 2)],
        vt[(1, 0)],
        vt[(1, 1)],
        vt[(1, 2)],
        vt[(2, 0)],
        vt[(2, 1)],
        vt[(2, 2)],
    );
    let sigma = Matrix3::new(s[0], 0.0, 0.0, 0.0, s[1], 0.0, 0.0, 0.0, s[2]);
    let u3 = Matrix3::new(
        u[(0, 0)],
        u[(0, 1)],
        u[(0, 2)],
        u[(1, 0)],
        u[(1, 1)],
        u[(1, 2)],
        u[(2, 0)],
        u[(2, 1)],
        u[(2, 2)],
    );

    let lattice = lattice_3d_lines(x, y, z, 6);
    let layer1 = apply_3d(&lattice, v_t, format!("{prefix}_svd_vt"));
    let layer2 = apply_3d(&lattice, sigma * v_t, format!("{prefix}_svd_sigma"));
    let layer3 = apply_3d(&lattice, u3 * sigma * v_t, format!("{prefix}_svd_u"));

    Ok(vec![layer1, layer2, layer3])
}

fn lattice_2d_lines(x: &AxisSpec, y: &AxisSpec, n: usize) -> Vec<([f64; 2], [f64; 2])> {
    let mut out = Vec::with_capacity(n * 2);
    for iy in 0..n {
        let t = iy as f64 / (n - 1) as f64;
        let yv = y.min + (y.max - y.min) * t;
        out.push(([x.min, yv], [x.max, yv]));
    }
    for ix in 0..n {
        let t = ix as f64 / (n - 1) as f64;
        let xv = x.min + (x.max - x.min) * t;
        out.push(([xv, y.min], [xv, y.max]));
    }
    out
}

fn lattice_3d_lines(
    x: &AxisSpec,
    y: &AxisSpec,
    z: &AxisSpec,
    n: usize,
) -> Vec<([f64; 3], [f64; 3])> {
    let mut out = Vec::with_capacity(n * n * 3);

    for iy in 0..n {
        let ty = iy as f64 / (n - 1) as f64;
        let yv = y.min + (y.max - y.min) * ty;
        for iz in 0..n {
            let tz = iz as f64 / (n - 1) as f64;
            let zv = z.min + (z.max - z.min) * tz;
            out.push(([x.min, yv, zv], [x.max, yv, zv]));
        }
    }

    for ix in 0..n {
        let tx = ix as f64 / (n - 1) as f64;
        let xv = x.min + (x.max - x.min) * tx;
        for iz in 0..n {
            let tz = iz as f64 / (n - 1) as f64;
            let zv = z.min + (z.max - z.min) * tz;
            out.push(([xv, y.min, zv], [xv, y.max, zv]));
        }
    }

    for ix in 0..n {
        let tx = ix as f64 / (n - 1) as f64;
        let xv = x.min + (x.max - x.min) * tx;
        for iy in 0..n {
            let ty = iy as f64 / (n - 1) as f64;
            let yv = y.min + (y.max - y.min) * ty;
            out.push(([xv, yv, z.min], [xv, yv, z.max]));
        }
    }

    out
}

fn apply_2d(base: &[([f64; 2], [f64; 2])], m: Matrix2<f64>, layer_id: String) -> GeometryBuffer {
    let mut vertices = Vec::with_capacity(base.len() * 2 * 3);
    let mut indices = Vec::with_capacity(base.len() * 3);
    let mut idx = 0u32;
    for &(a, b) in base {
        let va = m * Vector2::new(a[0], a[1]);
        let vb = m * Vector2::new(b[0], b[1]);
        vertices.extend_from_slice(&[va.x as f32, va.y as f32, 0.0, vb.x as f32, vb.y as f32, 0.0]);
        indices.extend_from_slice(&[idx, idx + 1, u32::MAX]);
        idx += 2;
    }
    GeometryBuffer {
        vertex_buffer: vertices,
        normal_buffer: Vec::new(),
        index_buffer: indices,
        uv_buffer: Vec::new(),
        layer_id,
        is_delta: false,
    }
}

fn apply_3d(base: &[([f64; 3], [f64; 3])], m: Matrix3<f64>, layer_id: String) -> GeometryBuffer {
    let mut vertices = Vec::with_capacity(base.len() * 2 * 3);
    let mut indices = Vec::with_capacity(base.len() * 3);
    let mut idx = 0u32;
    for &(a, b) in base {
        let va = m * Vector3::new(a[0], a[1], a[2]);
        let vb = m * Vector3::new(b[0], b[1], b[2]);
        vertices.extend_from_slice(&[
            va.x as f32,
            va.y as f32,
            va.z as f32,
            vb.x as f32,
            vb.y as f32,
            vb.z as f32,
        ]);
        indices.extend_from_slice(&[idx, idx + 1, u32::MAX]);
        idx += 2;
    }
    GeometryBuffer {
        vertex_buffer: vertices,
        normal_buffer: Vec::new(),
        index_buffer: indices,
        uv_buffer: Vec::new(),
        layer_id,
        is_delta: false,
    }
}
