use crate::{
    error::{MathvizError, MathvizResult},
    types::{AxisSpec, GeometryBuffer},
};

pub fn marching_cubes(
    x_axis: &AxisSpec,
    y_axis: &AxisSpec,
    z_axis: &AxisSpec,
    field: &[f64],
    layer_id: String,
) -> MathvizResult<GeometryBuffer> {
    let nx = x_axis.steps;
    let ny = y_axis.steps;
    let nz = z_axis.steps;
    let expected = nx * ny * nz;
    if field.len() != expected {
        return Err(MathvizError::MeshError(format!(
            "implicit field size mismatch: got {}, expected {}",
            field.len(), expected
        )));
    }

    let mut vertices = Vec::new();
    let mut indices = Vec::new();

    for iz in 0..nz.saturating_sub(1) {
        for iy in 0..ny.saturating_sub(1) {
            for ix in 0..nx.saturating_sub(1) {
                let center_idx = idx(ix, iy, iz, nx, ny);
                let v = field[center_idx];
                if !v.is_finite() || v >= 0.0 {
                    continue;
                }

                emit_face_if_boundary(
                    &mut vertices,
                    &mut indices,
                    x_axis,
                    y_axis,
                    z_axis,
                    field,
                    ix,
                    iy,
                    iz,
                    nx,
                    ny,
                    nz,
                    1,
                    0,
                    0,
                );
                emit_face_if_boundary(
                    &mut vertices,
                    &mut indices,
                    x_axis,
                    y_axis,
                    z_axis,
                    field,
                    ix,
                    iy,
                    iz,
                    nx,
                    ny,
                    nz,
                    -1,
                    0,
                    0,
                );
                emit_face_if_boundary(
                    &mut vertices,
                    &mut indices,
                    x_axis,
                    y_axis,
                    z_axis,
                    field,
                    ix,
                    iy,
                    iz,
                    nx,
                    ny,
                    nz,
                    0,
                    1,
                    0,
                );
                emit_face_if_boundary(
                    &mut vertices,
                    &mut indices,
                    x_axis,
                    y_axis,
                    z_axis,
                    field,
                    ix,
                    iy,
                    iz,
                    nx,
                    ny,
                    nz,
                    0,
                    -1,
                    0,
                );
                emit_face_if_boundary(
                    &mut vertices,
                    &mut indices,
                    x_axis,
                    y_axis,
                    z_axis,
                    field,
                    ix,
                    iy,
                    iz,
                    nx,
                    ny,
                    nz,
                    0,
                    0,
                    1,
                );
                emit_face_if_boundary(
                    &mut vertices,
                    &mut indices,
                    x_axis,
                    y_axis,
                    z_axis,
                    field,
                    ix,
                    iy,
                    iz,
                    nx,
                    ny,
                    nz,
                    0,
                    0,
                    -1,
                );
            }
        }
    }

    Ok(GeometryBuffer {
        vertex_buffer: vertices,
        normal_buffer: Vec::new(),
        index_buffer: indices,
        uv_buffer: Vec::new(),
        layer_id,
        is_delta: false,
    })
}

#[allow(clippy::too_many_arguments)]
fn emit_face_if_boundary(
    vertices: &mut Vec<f32>,
    indices: &mut Vec<u32>,
    x_axis: &AxisSpec,
    y_axis: &AxisSpec,
    z_axis: &AxisSpec,
    field: &[f64],
    ix: usize,
    iy: usize,
    iz: usize,
    nx: usize,
    ny: usize,
    nz: usize,
    dx: isize,
    dy: isize,
    dz: isize,
) {
    let nx_i = ix as isize + dx;
    let ny_i = iy as isize + dy;
    let nz_i = iz as isize + dz;

    let outside = nx_i < 0
        || ny_i < 0
        || nz_i < 0
        || nx_i >= (nx as isize)
        || ny_i >= (ny as isize)
        || nz_i >= (nz as isize)
        || field[idx(nx_i as usize, ny_i as usize, nz_i as usize, nx, ny)] >= 0.0;

    if !outside {
        return;
    }

    let x0 = x_axis.value_at(ix) as f32;
    let x1 = x_axis.value_at((ix + 1).min(x_axis.steps - 1)) as f32;
    let y0 = y_axis.value_at(iy) as f32;
    let y1 = y_axis.value_at((iy + 1).min(y_axis.steps - 1)) as f32;
    let z0 = z_axis.value_at(iz) as f32;
    let z1 = z_axis.value_at((iz + 1).min(z_axis.steps - 1)) as f32;

    let quad: [[f32; 3]; 4] = match (dx, dy, dz) {
        (1, 0, 0) => [[x1, y0, z0], [x1, y1, z0], [x1, y1, z1], [x1, y0, z1]],
        (-1, 0, 0) => [[x0, y0, z0], [x0, y0, z1], [x0, y1, z1], [x0, y1, z0]],
        (0, 1, 0) => [[x0, y1, z0], [x0, y1, z1], [x1, y1, z1], [x1, y1, z0]],
        (0, -1, 0) => [[x0, y0, z0], [x1, y0, z0], [x1, y0, z1], [x0, y0, z1]],
        (0, 0, 1) => [[x0, y0, z1], [x1, y0, z1], [x1, y1, z1], [x0, y1, z1]],
        _ => [[x0, y0, z0], [x0, y1, z0], [x1, y1, z0], [x1, y0, z0]],
    };

    let base = (vertices.len() / 3) as u32;
    for p in &quad {
        vertices.extend_from_slice(p);
    }
    indices.extend_from_slice(&[base, base + 1, base + 2, base, base + 2, base + 3]);
}

fn idx(ix: usize, iy: usize, iz: usize, nx: usize, ny: usize) -> usize {
    iz * (nx * ny) + iy * nx + ix
}
