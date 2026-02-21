use nalgebra::{Matrix2, Matrix3, Vector2, Vector3};

use crate::{
    error::{MathvizError, MathvizResult},
    types::{AxisSpec, GeometryBuffer},
};

pub fn generate_transform_layers(
    matrix: &[Vec<f64>],
    x_axis: &AxisSpec,
    y_axis: &AxisSpec,
    z_axis: Option<&AxisSpec>,
    grid_density: usize,
    layer_prefix: &str,
) -> MathvizResult<(GeometryBuffer, GeometryBuffer)> {
    if matrix.len() == 2 && matrix.iter().all(|row| row.len() == 2) {
        generate_2d_layers(matrix, x_axis, y_axis, grid_density, layer_prefix)
    } else if matrix.len() == 3 && matrix.iter().all(|row| row.len() == 3) {
        let z = z_axis.ok_or_else(|| {
            MathvizError::DomainViolation("3x3 transform requires z axis domain".to_string())
        })?;
        generate_3d_layers(matrix, x_axis, y_axis, z, grid_density, layer_prefix)
    } else {
        Err(MathvizError::DomainViolation(
            "matrix must be 2x2 or 3x3".to_string(),
        ))
    }
}

fn generate_2d_layers(
    matrix: &[Vec<f64>],
    x_axis: &AxisSpec,
    y_axis: &AxisSpec,
    density: usize,
    prefix: &str,
) -> MathvizResult<(GeometryBuffer, GeometryBuffer)> {
    let m = Matrix2::new(matrix[0][0], matrix[0][1], matrix[1][0], matrix[1][1]);
    let n = density.max(3);

    let mut before_vertices = Vec::new();
    let mut before_indices = Vec::new();
    let mut after_vertices = Vec::new();
    let mut after_indices = Vec::new();

    let mut before_idx = 0u32;
    let mut after_idx = 0u32;

    for i in 0..n {
        let t = i as f64 / (n - 1) as f64;
        let x = x_axis.min + (x_axis.max - x_axis.min) * t;
        let y0 = y_axis.min;
        let y1 = y_axis.max;

        let p0 = Vector2::new(x, y0);
        let p1 = Vector2::new(x, y1);
        push_line_2d(
            &mut before_vertices,
            &mut before_indices,
            &mut before_idx,
            p0,
            p1,
        );
        let tp0 = m * p0;
        let tp1 = m * p1;
        push_line_2d(
            &mut after_vertices,
            &mut after_indices,
            &mut after_idx,
            tp0,
            tp1,
        );
    }

    for i in 0..n {
        let t = i as f64 / (n - 1) as f64;
        let y = y_axis.min + (y_axis.max - y_axis.min) * t;
        let x0 = x_axis.min;
        let x1 = x_axis.max;

        let p0 = Vector2::new(x0, y);
        let p1 = Vector2::new(x1, y);
        push_line_2d(
            &mut before_vertices,
            &mut before_indices,
            &mut before_idx,
            p0,
            p1,
        );
        let tp0 = m * p0;
        let tp1 = m * p1;
        push_line_2d(
            &mut after_vertices,
            &mut after_indices,
            &mut after_idx,
            tp0,
            tp1,
        );
    }

    Ok((
        GeometryBuffer {
            vertex_buffer: before_vertices,
            normal_buffer: Vec::new(),
            index_buffer: before_indices,
            uv_buffer: Vec::new(),
            layer_id: format!("{prefix}_before"),
            is_delta: false,
        },
        GeometryBuffer {
            vertex_buffer: after_vertices,
            normal_buffer: Vec::new(),
            index_buffer: after_indices,
            uv_buffer: Vec::new(),
            layer_id: format!("{prefix}_after"),
            is_delta: false,
        },
    ))
}

fn generate_3d_layers(
    matrix: &[Vec<f64>],
    x_axis: &AxisSpec,
    y_axis: &AxisSpec,
    z_axis: &AxisSpec,
    density: usize,
    prefix: &str,
) -> MathvizResult<(GeometryBuffer, GeometryBuffer)> {
    let m = Matrix3::new(
        matrix[0][0],
        matrix[0][1],
        matrix[0][2],
        matrix[1][0],
        matrix[1][1],
        matrix[1][2],
        matrix[2][0],
        matrix[2][1],
        matrix[2][2],
    );
    let n = density.max(2);

    let mut before_vertices = Vec::new();
    let mut before_indices = Vec::new();
    let mut after_vertices = Vec::new();
    let mut after_indices = Vec::new();

    let mut before_idx = 0u32;
    let mut after_idx = 0u32;

    // Lines parallel to x-axis.
    for iy in 0..n {
        let ty = iy as f64 / (n - 1) as f64;
        let y = y_axis.min + (y_axis.max - y_axis.min) * ty;
        for iz in 0..n {
            let tz = iz as f64 / (n - 1) as f64;
            let z = z_axis.min + (z_axis.max - z_axis.min) * tz;
            let p0 = Vector3::new(x_axis.min, y, z);
            let p1 = Vector3::new(x_axis.max, y, z);
            push_line_3d(
                &mut before_vertices,
                &mut before_indices,
                &mut before_idx,
                p0,
                p1,
            );
            push_line_3d(
                &mut after_vertices,
                &mut after_indices,
                &mut after_idx,
                m * p0,
                m * p1,
            );
        }
    }

    // Lines parallel to y-axis.
    for ix in 0..n {
        let tx = ix as f64 / (n - 1) as f64;
        let x = x_axis.min + (x_axis.max - x_axis.min) * tx;
        for iz in 0..n {
            let tz = iz as f64 / (n - 1) as f64;
            let z = z_axis.min + (z_axis.max - z_axis.min) * tz;
            let p0 = Vector3::new(x, y_axis.min, z);
            let p1 = Vector3::new(x, y_axis.max, z);
            push_line_3d(
                &mut before_vertices,
                &mut before_indices,
                &mut before_idx,
                p0,
                p1,
            );
            push_line_3d(
                &mut after_vertices,
                &mut after_indices,
                &mut after_idx,
                m * p0,
                m * p1,
            );
        }
    }

    // Lines parallel to z-axis.
    for ix in 0..n {
        let tx = ix as f64 / (n - 1) as f64;
        let x = x_axis.min + (x_axis.max - x_axis.min) * tx;
        for iy in 0..n {
            let ty = iy as f64 / (n - 1) as f64;
            let y = y_axis.min + (y_axis.max - y_axis.min) * ty;
            let p0 = Vector3::new(x, y, z_axis.min);
            let p1 = Vector3::new(x, y, z_axis.max);
            push_line_3d(
                &mut before_vertices,
                &mut before_indices,
                &mut before_idx,
                p0,
                p1,
            );
            push_line_3d(
                &mut after_vertices,
                &mut after_indices,
                &mut after_idx,
                m * p0,
                m * p1,
            );
        }
    }

    Ok((
        GeometryBuffer {
            vertex_buffer: before_vertices,
            normal_buffer: Vec::new(),
            index_buffer: before_indices,
            uv_buffer: Vec::new(),
            layer_id: format!("{prefix}_before"),
            is_delta: false,
        },
        GeometryBuffer {
            vertex_buffer: after_vertices,
            normal_buffer: Vec::new(),
            index_buffer: after_indices,
            uv_buffer: Vec::new(),
            layer_id: format!("{prefix}_after"),
            is_delta: false,
        },
    ))
}

fn push_line_2d(
    vertices: &mut Vec<f32>,
    indices: &mut Vec<u32>,
    idx: &mut u32,
    a: Vector2<f64>,
    b: Vector2<f64>,
) {
    let base = *idx;
    vertices.extend_from_slice(&[a.x as f32, a.y as f32, 0.0, b.x as f32, b.y as f32, 0.0]);
    indices.extend_from_slice(&[base, base + 1, u32::MAX]);
    *idx += 2;
}

fn push_line_3d(
    vertices: &mut Vec<f32>,
    indices: &mut Vec<u32>,
    idx: &mut u32,
    a: Vector3<f64>,
    b: Vector3<f64>,
) {
    let base = *idx;
    vertices.extend_from_slice(&[
        a.x as f32, a.y as f32, a.z as f32, b.x as f32, b.y as f32, b.z as f32,
    ]);
    indices.extend_from_slice(&[base, base + 1, u32::MAX]);
    *idx += 2;
}
