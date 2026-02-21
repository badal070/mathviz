use crate::types::AxisSpec;

pub fn compute_divergence_and_curl(
    p: &[f64],
    q: &[f64],
    r: &[f64],
    x: &AxisSpec,
    y: &AxisSpec,
    z: &AxisSpec,
) -> (Vec<f32>, Vec<f32>) {
    let nx = x.steps;
    let ny = y.steps;
    let nz = z.steps;
    let dims = GridDims { nx, ny, nz };
    let n = nx * ny * nz;

    let dx = x.spacing();
    let dy = y.spacing();
    let dz = z.spacing();

    let mut divergence = vec![0.0f32; n];
    let mut curl = vec![0.0f32; n * 3];

    for iz in 0..nz {
        for iy in 0..ny {
            for ix in 0..nx {
                let idx = index(ix, iy, iz, nx, ny);

                let d_p_dx = diff_axis(p, [ix, iy, iz], dims, dx, Axis::X);
                let d_q_dy = diff_axis(q, [ix, iy, iz], dims, dy, Axis::Y);
                let d_r_dz = diff_axis(r, [ix, iy, iz], dims, dz, Axis::Z);

                let d_r_dy = diff_axis(r, [ix, iy, iz], dims, dy, Axis::Y);
                let d_q_dz = diff_axis(q, [ix, iy, iz], dims, dz, Axis::Z);
                let d_p_dz = diff_axis(p, [ix, iy, iz], dims, dz, Axis::Z);
                let d_r_dx = diff_axis(r, [ix, iy, iz], dims, dx, Axis::X);
                let d_q_dx = diff_axis(q, [ix, iy, iz], dims, dx, Axis::X);
                let d_p_dy = diff_axis(p, [ix, iy, iz], dims, dy, Axis::Y);

                divergence[idx] = (d_p_dx + d_q_dy + d_r_dz) as f32;
                curl[idx * 3] = (d_r_dy - d_q_dz) as f32;
                curl[idx * 3 + 1] = (d_p_dz - d_r_dx) as f32;
                curl[idx * 3 + 2] = (d_q_dx - d_p_dy) as f32;
            }
        }
    }

    (divergence, curl)
}

#[derive(Copy, Clone)]
enum Axis {
    X,
    Y,
    Z,
}

#[derive(Copy, Clone)]
struct GridDims {
    nx: usize,
    ny: usize,
    nz: usize,
}

fn diff_axis(
    field: &[f64],
    ijk: [usize; 3],
    dims: GridDims,
    h: f64,
    axis: Axis,
) -> f64 {
    let [ix, iy, iz] = ijk;
    let nx = dims.nx;
    let ny = dims.ny;
    let nz = dims.nz;

    match axis {
        Axis::X => {
            if ix == 0 {
                (field[index(ix + 1, iy, iz, nx, ny)] - field[index(ix, iy, iz, nx, ny)]) / h
            } else if ix == nx - 1 {
                (field[index(ix, iy, iz, nx, ny)] - field[index(ix - 1, iy, iz, nx, ny)]) / h
            } else {
                (field[index(ix + 1, iy, iz, nx, ny)] - field[index(ix - 1, iy, iz, nx, ny)]) / (2.0 * h)
            }
        }
        Axis::Y => {
            if iy == 0 {
                (field[index(ix, iy + 1, iz, nx, ny)] - field[index(ix, iy, iz, nx, ny)]) / h
            } else if iy == ny - 1 {
                (field[index(ix, iy, iz, nx, ny)] - field[index(ix, iy - 1, iz, nx, ny)]) / h
            } else {
                (field[index(ix, iy + 1, iz, nx, ny)] - field[index(ix, iy - 1, iz, nx, ny)]) / (2.0 * h)
            }
        }
        Axis::Z => {
            if iz == 0 {
                (field[index(ix, iy, iz + 1, nx, ny)] - field[index(ix, iy, iz, nx, ny)]) / h
            } else if iz == nz - 1 {
                (field[index(ix, iy, iz, nx, ny)] - field[index(ix, iy, iz - 1, nx, ny)]) / h
            } else {
                (field[index(ix, iy, iz + 1, nx, ny)] - field[index(ix, iy, iz - 1, nx, ny)]) / (2.0 * h)
            }
        }
    }
}

fn index(ix: usize, iy: usize, iz: usize, nx: usize, ny: usize) -> usize {
    iz * (nx * ny) + iy * nx + ix
}
