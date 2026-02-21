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

fn svd_2d(matrix: &[Vec<f64>], x: &AxisSpec, y: &AxisSpec, prefix: &str) -> MathvizResult<Vec<GeometryBuffer>> {
    let m = DMatrix::from_row_slice(2, 2, &[matrix[0][0], matrix[0][1], matrix[1][0], matrix[1][1]]);
    let svd = m.svd(true, true);
    let u = svd.u.ok_or_else(|| MathvizError::EvalError("SVD U missing".to_string()))?;
    let vt = svd.v_t.ok_or_else(|| MathvizError::EvalError("SVD Vt missing".to_string()))?;
    let s = svd.singular_values;

    let v_t = Matrix2::new(vt[(0, 0)], vt[(0, 1)], vt[(1, 0)], vt[(1, 1)]);
    let sigma = Matrix2::new(s[0], 0.0, 0.0, s[1]);
    let u2 = Matrix2::new(u[(0, 0)], u[(0, 1)], u[(1, 0)], u[(1, 1)]);

    let base = grid_2d_vertices(x, y, 12);
    let layer1 = apply_2d(&base, v_t, format!("{prefix}_svd_vt"));
    let layer2 = apply_2d(&base, sigma * v_t, format!("{prefix}_svd_sigma"));
    let layer3 = apply_2d(&base, u2 * sigma * v_t, format!("{prefix}_svd_u"));

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
            matrix[0][0], matrix[0][1], matrix[0][2], matrix[1][0], matrix[1][1], matrix[1][2], matrix[2][0],
            matrix[2][1], matrix[2][2],
        ],
    );
    let svd = m.svd(true, true);
    let u = svd.u.ok_or_else(|| MathvizError::EvalError("SVD U missing".to_string()))?;
    let vt = svd.v_t.ok_or_else(|| MathvizError::EvalError("SVD Vt missing".to_string()))?;
    let s = svd.singular_values;

    let v_t = Matrix3::new(
        vt[(0, 0)], vt[(0, 1)], vt[(0, 2)], vt[(1, 0)], vt[(1, 1)], vt[(1, 2)], vt[(2, 0)], vt[(2, 1)],
        vt[(2, 2)],
    );
    let sigma = Matrix3::new(s[0], 0.0, 0.0, 0.0, s[1], 0.0, 0.0, 0.0, s[2]);
    let u3 = Matrix3::new(
        u[(0, 0)], u[(0, 1)], u[(0, 2)], u[(1, 0)], u[(1, 1)], u[(1, 2)], u[(2, 0)], u[(2, 1)], u[(2, 2)],
    );

    let base = grid_3d_vertices(x, y, z, 6);
    let layer1 = apply_3d(&base, v_t, format!("{prefix}_svd_vt"));
    let layer2 = apply_3d(&base, sigma * v_t, format!("{prefix}_svd_sigma"));
    let layer3 = apply_3d(&base, u3 * sigma * v_t, format!("{prefix}_svd_u"));

    Ok(vec![layer1, layer2, layer3])
}

fn grid_2d_vertices(x: &AxisSpec, y: &AxisSpec, n: usize) -> Vec<[f64; 2]> {
    let mut out = Vec::with_capacity(n * n);
    for iy in 0..n {
        let ty = iy as f64 / (n - 1) as f64;
        let yv = y.min + (y.max - y.min) * ty;
        for ix in 0..n {
            let tx = ix as f64 / (n - 1) as f64;
            let xv = x.min + (x.max - x.min) * tx;
            out.push([xv, yv]);
        }
    }
    out
}

fn grid_3d_vertices(x: &AxisSpec, y: &AxisSpec, z: &AxisSpec, n: usize) -> Vec<[f64; 3]> {
    let mut out = Vec::with_capacity(n * n * n);
    for iz in 0..n {
        let tz = iz as f64 / (n - 1) as f64;
        let zv = z.min + (z.max - z.min) * tz;
        for iy in 0..n {
            let ty = iy as f64 / (n - 1) as f64;
            let yv = y.min + (y.max - y.min) * ty;
            for ix in 0..n {
                let tx = ix as f64 / (n - 1) as f64;
                let xv = x.min + (x.max - x.min) * tx;
                out.push([xv, yv, zv]);
            }
        }
    }
    out
}

fn apply_2d(base: &[[f64; 2]], m: Matrix2<f64>, layer_id: String) -> GeometryBuffer {
    let mut vertices = Vec::with_capacity(base.len() * 3);
    let mut indices = Vec::with_capacity(base.len());
    for (i, p) in base.iter().enumerate() {
        let v = m * Vector2::new(p[0], p[1]);
        vertices.extend_from_slice(&[v.x as f32, v.y as f32, 0.0]);
        indices.push(i as u32);
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

fn apply_3d(base: &[[f64; 3]], m: Matrix3<f64>, layer_id: String) -> GeometryBuffer {
    let mut vertices = Vec::with_capacity(base.len() * 3);
    let mut indices = Vec::with_capacity(base.len());
    for (i, p) in base.iter().enumerate() {
        let v = m * Vector3::new(p[0], p[1], p[2]);
        vertices.extend_from_slice(&[v.x as f32, v.y as f32, v.z as f32]);
        indices.push(i as u32);
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
