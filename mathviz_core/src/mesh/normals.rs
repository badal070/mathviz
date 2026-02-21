pub fn compute_explicit_normals(
    z_values: &[f64],
    nx: usize,
    ny: usize,
    dx: f64,
    dy: f64,
) -> Vec<f32> {
    let mut normals = vec![0.0f32; nx * ny * 3];

    for iy in 0..ny {
        for ix in 0..nx {
            let idx = iy * nx + ix;

            let ix_prev = ix.saturating_sub(1);
            let ix_next = (ix + 1).min(nx - 1);
            let iy_prev = iy.saturating_sub(1);
            let iy_next = (iy + 1).min(ny - 1);

            let z_l = z_values[iy * nx + ix_prev];
            let z_r = z_values[iy * nx + ix_next];
            let z_b = z_values[iy_prev * nx + ix];
            let z_t = z_values[iy_next * nx + ix];

            let dfdx = (z_r - z_l) / ((ix_next as f64 - ix_prev as f64).max(1.0) * dx);
            let dfdy = (z_t - z_b) / ((iy_next as f64 - iy_prev as f64).max(1.0) * dy);

            let mut nxv = -dfdx;
            let mut nyv = -dfdy;
            let mut nzv = 1.0;

            let norm = (nxv * nxv + nyv * nyv + nzv * nzv).sqrt();
            if norm > 0.0 {
                nxv /= norm;
                nyv /= norm;
                nzv /= norm;
            }

            normals[idx * 3] = nxv as f32;
            normals[idx * 3 + 1] = nyv as f32;
            normals[idx * 3 + 2] = nzv as f32;
        }
    }

    normals
}
