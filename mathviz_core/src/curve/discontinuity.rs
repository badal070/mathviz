pub fn discontinuity_breaks(vertices: &[f32], factor: f64) -> Vec<bool> {
    let n = vertices.len() / 3;
    if n < 3 {
        return vec![false; n];
    }

    let mut slope_mag = Vec::with_capacity(n - 1);
    for i in 1..n {
        let dx = (vertices[i * 3] - vertices[(i - 1) * 3]) as f64;
        let dy = (vertices[i * 3 + 1] - vertices[(i - 1) * 3 + 1]) as f64;
        let dz = (vertices[i * 3 + 2] - vertices[(i - 1) * 3 + 2]) as f64;
        let denom = dx.abs().max(1e-12);
        slope_mag.push((dy.abs() + dz.abs()) / denom);
    }

    let mut sorted = slope_mag.clone();
    sorted.sort_by(|a, b| a.total_cmp(b));
    let median = sorted[sorted.len() / 2].max(1e-12);
    let threshold = median * factor.max(1.0);

    let mut breaks = vec![false; n];
    for i in 1..n {
        if slope_mag[i - 1] > threshold || !slope_mag[i - 1].is_finite() {
            breaks[i] = true;
        }
    }
    breaks
}
