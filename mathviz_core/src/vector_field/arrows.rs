use nalgebra::{Unit, UnitQuaternion, Vector3};

use crate::{
    error::{MathvizError, MathvizResult},
    types::{ArrowInstanceBuffer, GeometryBuffer, Point3},
};

pub fn unit_arrow_geometry(layer_id: String) -> GeometryBuffer {
    // Minimal unit arrow: shaft quad strip approximation + cone tip triangles.
    // Geometry is centered at origin and points along +Z.
    let r = 0.05f32;
    let h_shaft = 0.75f32;
    let h_tip = 0.25f32;

    let mut vertices = Vec::with_capacity(8 * 3 + 5 * 3);
    let mut indices = Vec::new();

    let ring = [
        (r, 0.0),
        (0.0, r),
        (-r, 0.0),
        (0.0, -r),
    ];

    for &(x, y) in &ring {
        vertices.extend_from_slice(&[x, y, 0.0]);
    }
    for &(x, y) in &ring {
        vertices.extend_from_slice(&[x, y, h_shaft]);
    }

    for i in 0..4u32 {
        let j = (i + 1) % 4;
        let b0 = i;
        let b1 = j;
        let t0 = i + 4;
        let t1 = j + 4;
        indices.extend_from_slice(&[b0, b1, t1, b0, t1, t0]);
    }

    let cone_base_start = (vertices.len() / 3) as u32;
    for &(x, y) in &ring {
        vertices.extend_from_slice(&[x * 1.6, y * 1.6, h_shaft]);
    }
    let tip_idx = (vertices.len() / 3) as u32;
    vertices.extend_from_slice(&[0.0, 0.0, h_shaft + h_tip]);

    for i in 0..4u32 {
        let j = (i + 1) % 4;
        indices.extend_from_slice(&[cone_base_start + i, cone_base_start + j, tip_idx]);
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

pub fn build_instances(points: &[Point3], vectors: &[[f64; 3]]) -> MathvizResult<ArrowInstanceBuffer> {
    if points.len() != vectors.len() {
        return Err(MathvizError::EvalError(
            "points/vectors size mismatch for arrow instances".to_string(),
        ));
    }

    let mut mags: Vec<f64> = vectors
        .iter()
        .map(|v| (v[0] * v[0] + v[1] * v[1] + v[2] * v[2]).sqrt())
        .filter(|m| m.is_finite())
        .collect();
    if mags.is_empty() {
        mags.push(1.0);
    }
    mags.sort_by(|a, b| a.total_cmp(b));
    let p90_idx = ((mags.len() as f64) * 0.9).floor() as usize;
    let p90 = mags[p90_idx.min(mags.len().saturating_sub(1))].max(1e-12);

    let mut data = Vec::with_capacity(points.len() * 8);
    let z_axis = Unit::new_normalize(Vector3::new(0.0, 0.0, 1.0));

    for (p, v) in points.iter().zip(vectors.iter()) {
        let dir = Vector3::new(v[0], v[1], v[2]);
        let mag = dir.norm();
        let scale = (mag / p90).min(2.0) as f32;

        let q = if mag <= 1e-12 || !mag.is_finite() {
            UnitQuaternion::identity()
        } else {
            let target = Unit::new_normalize(dir);
            UnitQuaternion::rotation_between_axis(&z_axis, &target).unwrap_or_else(UnitQuaternion::identity)
        };

        data.extend_from_slice(&[
            p.x as f32,
            p.y as f32,
            p.z as f32,
            q.i as f32,
            q.j as f32,
            q.k as f32,
            q.w as f32,
            scale,
        ]);
    }

    Ok(ArrowInstanceBuffer { data })
}
