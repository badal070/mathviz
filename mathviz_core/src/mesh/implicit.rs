use std::collections::HashMap;

use crate::{
    error::{MathvizError, MathvizResult},
    types::{AxisSpec, GeometryBuffer},
};

const ISO_LEVEL: f64 = 0.0;

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
            field.len(),
            expected
        )));
    }

    if nx < 2 || ny < 2 || nz < 2 {
        return Err(MathvizError::MeshError(
            "implicit surface requires at least 2 steps on x/y/z".to_string(),
        ));
    }

    let spacing = x_axis
        .spacing()
        .abs()
        .min(y_axis.spacing().abs())
        .min(z_axis.spacing().abs());
    let quant_eps = spacing.max(1e-9) * 1e-5;

    let mut vertex_map: HashMap<(i64, i64, i64), u32> = HashMap::new();
    let mut vertices: Vec<[f32; 3]> = Vec::new();
    let mut indices: Vec<u32> = Vec::new();

    // Cube split into 6 tetrahedra, consistent winding across adjacent cubes.
    const TETS: [[usize; 4]; 6] = [
        [0, 5, 1, 6],
        [0, 5, 6, 4],
        [0, 1, 2, 6],
        [0, 2, 3, 6],
        [0, 6, 7, 4],
        [0, 3, 7, 6],
    ];

    for iz in 0..(nz - 1) {
        for iy in 0..(ny - 1) {
            for ix in 0..(nx - 1) {
                let corners = cube_corners(ix, iy, iz, x_axis, y_axis, z_axis, field, nx, ny);

                for tet in TETS {
                    let tris = tetra_isosurface(&corners, tet);
                    for tri in tris {
                        let i0 =
                            get_or_insert_vertex(tri[0], quant_eps, &mut vertex_map, &mut vertices);
                        let i1 =
                            get_or_insert_vertex(tri[1], quant_eps, &mut vertex_map, &mut vertices);
                        let i2 =
                            get_or_insert_vertex(tri[2], quant_eps, &mut vertex_map, &mut vertices);

                        if i0 == i1 || i1 == i2 || i0 == i2 {
                            continue;
                        }

                        indices.extend_from_slice(&[i0, i1, i2]);
                    }
                }
            }
        }
    }

    let normals = smooth_vertex_normals(&vertices, &indices);

    let mut vertex_buffer = Vec::with_capacity(vertices.len() * 3);
    for p in &vertices {
        vertex_buffer.extend_from_slice(p);
    }

    Ok(GeometryBuffer {
        vertex_buffer,
        normal_buffer: normals,
        index_buffer: indices,
        uv_buffer: Vec::new(),
        layer_id,
        is_delta: false,
    })
}

#[derive(Copy, Clone)]
struct Corner {
    pos: [f32; 3],
    val: f64,
}

#[allow(clippy::too_many_arguments)]
fn cube_corners(
    ix: usize,
    iy: usize,
    iz: usize,
    x_axis: &AxisSpec,
    y_axis: &AxisSpec,
    z_axis: &AxisSpec,
    field: &[f64],
    nx: usize,
    ny: usize,
) -> [Corner; 8] {
    let x0 = x_axis.value_at(ix) as f32;
    let x1 = x_axis.value_at(ix + 1) as f32;
    let y0 = y_axis.value_at(iy) as f32;
    let y1 = y_axis.value_at(iy + 1) as f32;
    let z0 = z_axis.value_at(iz) as f32;
    let z1 = z_axis.value_at(iz + 1) as f32;

    [
        Corner {
            pos: [x0, y0, z0],
            val: field[idx(ix, iy, iz, nx, ny)],
        },
        Corner {
            pos: [x1, y0, z0],
            val: field[idx(ix + 1, iy, iz, nx, ny)],
        },
        Corner {
            pos: [x1, y1, z0],
            val: field[idx(ix + 1, iy + 1, iz, nx, ny)],
        },
        Corner {
            pos: [x0, y1, z0],
            val: field[idx(ix, iy + 1, iz, nx, ny)],
        },
        Corner {
            pos: [x0, y0, z1],
            val: field[idx(ix, iy, iz + 1, nx, ny)],
        },
        Corner {
            pos: [x1, y0, z1],
            val: field[idx(ix + 1, iy, iz + 1, nx, ny)],
        },
        Corner {
            pos: [x1, y1, z1],
            val: field[idx(ix + 1, iy + 1, iz + 1, nx, ny)],
        },
        Corner {
            pos: [x0, y1, z1],
            val: field[idx(ix, iy + 1, iz + 1, nx, ny)],
        },
    ]
}

fn tetra_isosurface(corners: &[Corner; 8], tet: [usize; 4]) -> Vec<[[f32; 3]; 3]> {
    let mut inside = Vec::with_capacity(4);
    let mut outside = Vec::with_capacity(4);

    for &i in &tet {
        if corners[i].val < ISO_LEVEL {
            inside.push(i);
        } else {
            outside.push(i);
        }
    }

    match inside.len() {
        0 | 4 => Vec::new(),
        1 => {
            let i = inside[0];
            let p0 = interpolate(corners[i], corners[outside[0]]);
            let p1 = interpolate(corners[i], corners[outside[1]]);
            let p2 = interpolate(corners[i], corners[outside[2]]);
            vec![[p0, p1, p2]]
        }
        3 => {
            let o = outside[0];
            let p0 = interpolate(corners[o], corners[inside[0]]);
            let p1 = interpolate(corners[o], corners[inside[1]]);
            let p2 = interpolate(corners[o], corners[inside[2]]);
            vec![[p0, p2, p1]]
        }
        2 => {
            let i0 = inside[0];
            let i1 = inside[1];
            let o0 = outside[0];
            let o1 = outside[1];

            let p0 = interpolate(corners[i0], corners[o0]);
            let p1 = interpolate(corners[i1], corners[o0]);
            let p2 = interpolate(corners[i1], corners[o1]);
            let p3 = interpolate(corners[i0], corners[o1]);

            vec![[p0, p1, p2], [p0, p2, p3]]
        }
        _ => Vec::new(),
    }
}

fn interpolate(a: Corner, b: Corner) -> [f32; 3] {
    let denom = b.val - a.val;
    let t = if denom.abs() < 1e-15 {
        0.5
    } else {
        ((ISO_LEVEL - a.val) / denom).clamp(0.0, 1.0)
    } as f32;

    [
        a.pos[0] + (b.pos[0] - a.pos[0]) * t,
        a.pos[1] + (b.pos[1] - a.pos[1]) * t,
        a.pos[2] + (b.pos[2] - a.pos[2]) * t,
    ]
}

fn get_or_insert_vertex(
    pos: [f32; 3],
    eps: f64,
    map: &mut HashMap<(i64, i64, i64), u32>,
    vertices: &mut Vec<[f32; 3]>,
) -> u32 {
    let key = quant_key(pos, eps);
    if let Some(&idx) = map.get(&key) {
        return idx;
    }

    let idx = vertices.len() as u32;
    vertices.push(pos);
    map.insert(key, idx);
    idx
}

fn quant_key(pos: [f32; 3], eps: f64) -> (i64, i64, i64) {
    (
        (pos[0] as f64 / eps).round() as i64,
        (pos[1] as f64 / eps).round() as i64,
        (pos[2] as f64 / eps).round() as i64,
    )
}

fn smooth_vertex_normals(vertices: &[[f32; 3]], indices: &[u32]) -> Vec<f32> {
    let mut accum = vec![[0.0f64; 3]; vertices.len()];

    for tri in indices.chunks_exact(3) {
        let i0 = tri[0] as usize;
        let i1 = tri[1] as usize;
        let i2 = tri[2] as usize;

        if i0 >= vertices.len() || i1 >= vertices.len() || i2 >= vertices.len() {
            continue;
        }

        let p0 = vertices[i0];
        let p1 = vertices[i1];
        let p2 = vertices[i2];

        let ux = (p1[0] - p0[0]) as f64;
        let uy = (p1[1] - p0[1]) as f64;
        let uz = (p1[2] - p0[2]) as f64;
        let vx = (p2[0] - p0[0]) as f64;
        let vy = (p2[1] - p0[1]) as f64;
        let vz = (p2[2] - p0[2]) as f64;

        let nx = uy * vz - uz * vy;
        let ny = uz * vx - ux * vz;
        let nz = ux * vy - uy * vx;
        let n2 = nx * nx + ny * ny + nz * nz;
        if !n2.is_finite() || n2 <= 1e-30 {
            continue;
        }

        for &i in &[i0, i1, i2] {
            accum[i][0] += nx;
            accum[i][1] += ny;
            accum[i][2] += nz;
        }
    }

    let mut out = Vec::with_capacity(vertices.len() * 3);
    for n in accum {
        let len = (n[0] * n[0] + n[1] * n[1] + n[2] * n[2]).sqrt();
        if len > 1e-30 {
            out.extend_from_slice(&[
                (n[0] / len) as f32,
                (n[1] / len) as f32,
                (n[2] / len) as f32,
            ]);
        } else {
            out.extend_from_slice(&[0.0, 0.0, 1.0]);
        }
    }
    out
}

fn idx(ix: usize, iy: usize, iz: usize, nx: usize, ny: usize) -> usize {
    iz * (nx * ny) + iy * nx + ix
}
