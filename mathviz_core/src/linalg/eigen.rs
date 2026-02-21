use nalgebra::{Matrix3, Vector2, Vector3};

use crate::{
    error::{MathvizError, MathvizResult},
    types::GeometryBuffer,
};

pub fn eigen_layers(matrix: &[Vec<f64>], layer_prefix: &str) -> MathvizResult<Vec<GeometryBuffer>> {
    if matrix.len() == 2 && matrix.iter().all(|r| r.len() == 2) {
        eigen_2x2_layers(matrix, layer_prefix)
    } else if matrix.len() == 3 && matrix.iter().all(|r| r.len() == 3) {
        eigen_3x3_layers(matrix, layer_prefix)
    } else {
        Err(MathvizError::DomainViolation(
            "matrix must be 2x2 or 3x3 for eigen decomposition".to_string(),
        ))
    }
}

fn eigen_2x2_layers(matrix: &[Vec<f64>], prefix: &str) -> MathvizResult<Vec<GeometryBuffer>> {
    let a = matrix[0][0];
    let b = matrix[0][1];
    let c = matrix[1][0];
    let d = matrix[1][1];

    let tr = a + d;
    let det = a * d - b * c;
    let disc = tr * tr - 4.0 * det;

    if disc < 0.0 {
        return Ok(vec![rotation_scale_indicator_2d(
            a,
            b,
            c,
            d,
            format!("{prefix}_eigen_complex"),
        )]);
    }

    let sqrt_disc = disc.sqrt();
    let l1 = 0.5 * (tr + sqrt_disc);
    let l2 = 0.5 * (tr - sqrt_disc);

    let v1 = eigenvector_2x2(a, b, c, d, l1);
    let v2 = eigenvector_2x2(a, b, c, d, l2);

    Ok(vec![
        arrow_layer_2d(v1, l1, format!("{prefix}_eigen_0")),
        arrow_layer_2d(v2, l2, format!("{prefix}_eigen_1")),
    ])
}

fn eigen_3x3_layers(matrix: &[Vec<f64>], prefix: &str) -> MathvizResult<Vec<GeometryBuffer>> {
    let mut a = Matrix3::new(
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
    let mut q_total = Matrix3::identity();

    for _ in 0..200 {
        let qr = a.qr();
        let q = qr.q();
        let r = qr.r();
        a = r * q;
        q_total *= q;
    }

    let lambdas = [a[(0, 0)], a[(1, 1)], a[(2, 2)]];
    let mut out = Vec::with_capacity(3);
    for i in 0..3 {
        let vec = Vector3::new(q_total[(0, i)], q_total[(1, i)], q_total[(2, i)]);
        out.push(arrow_layer_3d(
            vec,
            lambdas[i],
            format!("{prefix}_eigen_{i}"),
        ));
    }
    Ok(out)
}

fn eigenvector_2x2(a: f64, b: f64, c: f64, d: f64, l: f64) -> Vector2<f64> {
    let m00 = a - l;
    let m11 = d - l;

    let v = if b.abs() > c.abs() {
        Vector2::new(-b, m00)
    } else {
        Vector2::new(m11, -c)
    };

    let n = v.norm();
    if n <= 1e-12 {
        Vector2::new(1.0, 0.0)
    } else {
        v / n
    }
}

fn arrow_layer_2d(v: Vector2<f64>, lambda: f64, layer_id: String) -> GeometryBuffer {
    let scale = lambda.abs().max(0.1);
    let p0 = -v * scale;
    let p1 = v * scale;

    GeometryBuffer {
        vertex_buffer: vec![p0.x as f32, p0.y as f32, 0.0, p1.x as f32, p1.y as f32, 0.0],
        normal_buffer: Vec::new(),
        index_buffer: vec![0, 1],
        uv_buffer: Vec::new(),
        layer_id,
        is_delta: false,
    }
}

fn arrow_layer_3d(v: Vector3<f64>, lambda: f64, layer_id: String) -> GeometryBuffer {
    let n = v.norm();
    let dir = if n <= 1e-12 {
        Vector3::new(1.0, 0.0, 0.0)
    } else {
        v / n
    };
    let scale = lambda.abs().max(0.1);
    let p0 = -dir * scale;
    let p1 = dir * scale;

    GeometryBuffer {
        vertex_buffer: vec![
            p0.x as f32,
            p0.y as f32,
            p0.z as f32,
            p1.x as f32,
            p1.y as f32,
            p1.z as f32,
        ],
        normal_buffer: Vec::new(),
        index_buffer: vec![0, 1],
        uv_buffer: Vec::new(),
        layer_id,
        is_delta: false,
    }
}

fn rotation_scale_indicator_2d(a: f64, b: f64, c: f64, d: f64, layer_id: String) -> GeometryBuffer {
    let m = nalgebra::Matrix2::new(a, b, c, d);
    let samples = 96usize;

    let mut vertices = Vec::with_capacity(samples * 3);
    let mut indices = Vec::with_capacity(samples + 2);

    for i in 0..samples {
        let theta = (i as f64 / samples as f64) * std::f64::consts::TAU;
        let p = Vector2::new(theta.cos(), theta.sin());
        let q = m * p;
        vertices.extend_from_slice(&[q.x as f32, q.y as f32, 0.0]);
        indices.push(i as u32);
    }
    // Close the loop and terminate line-strip segment.
    indices.push(0);
    indices.push(u32::MAX);

    GeometryBuffer {
        vertex_buffer: vertices,
        normal_buffer: Vec::new(),
        index_buffer: indices,
        uv_buffer: Vec::new(),
        layer_id,
        is_delta: false,
    }
}
