use crate::{
    error::{MathvizError, MathvizResult},
    mesh::normals::compute_explicit_normals,
    types::{AxisSpec, GeometryBuffer},
};

pub fn build_explicit_surface(
    x_axis: &AxisSpec,
    y_axis: &AxisSpec,
    z_values: &[f64],
    render_bound: f64,
    layer_id: String,
) -> MathvizResult<GeometryBuffer> {
    let nx = x_axis.steps;
    let ny = y_axis.steps;
    let expected = nx * ny;
    if z_values.len() != expected {
        return Err(MathvizError::MeshError(format!(
            "z field size mismatch: got {}, expected {}",
            z_values.len(),
            expected
        )));
    }

    let mut vertices = Vec::with_capacity(expected * 3);
    let mut z_clamped = Vec::with_capacity(expected);

    for iy in 0..ny {
        for ix in 0..nx {
            let x = x_axis.value_at(ix);
            let y = y_axis.value_at(iy);
            let z_in = z_values[iy * nx + ix];
            let z = if z_in.is_finite() {
                z_in.clamp(-render_bound, render_bound)
            } else {
                z_in
            };

            vertices.push(x as f32);
            vertices.push(y as f32);
            vertices.push(z as f32);
            z_clamped.push(z);
        }
    }

    let mut indices = Vec::with_capacity((nx - 1) * (ny - 1) * 6);
    for iy in 0..(ny - 1) {
        for ix in 0..(nx - 1) {
            let i0 = (iy * nx + ix) as u32;
            let i1 = (iy * nx + ix + 1) as u32;
            let i2 = ((iy + 1) * nx + ix) as u32;
            let i3 = ((iy + 1) * nx + ix + 1) as u32;

            if tri_is_valid(&vertices, i0, i1, i2) {
                indices.extend_from_slice(&[i0, i1, i2]);
            }
            if tri_is_valid(&vertices, i1, i3, i2) {
                indices.extend_from_slice(&[i1, i3, i2]);
            }
        }
    }

    let normals = compute_explicit_normals(&z_clamped, nx, ny, x_axis.spacing(), y_axis.spacing());

    Ok(GeometryBuffer {
        vertex_buffer: vertices,
        normal_buffer: normals,
        index_buffer: indices,
        uv_buffer: Vec::new(),
        layer_id,
        is_delta: false,
    })
}

fn tri_is_valid(vertices: &[f32], i0: u32, i1: u32, i2: u32) -> bool {
    let v0 = &vertices[(i0 as usize) * 3..(i0 as usize) * 3 + 3];
    let v1 = &vertices[(i1 as usize) * 3..(i1 as usize) * 3 + 3];
    let v2 = &vertices[(i2 as usize) * 3..(i2 as usize) * 3 + 3];

    if !(v0.iter().all(|v| v.is_finite())
        && v1.iter().all(|v| v.is_finite())
        && v2.iter().all(|v| v.is_finite()))
    {
        return false;
    }

    !(v0 == v1 || v1 == v2 || v0 == v2)
}
