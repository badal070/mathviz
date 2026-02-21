pub fn discontinuity_breaks(vertices: &[f32], factor: f64) -> Vec<bool> {
    let n = vertices.len() / 3;
    if n < 3 {
        return vec![false; n];
    }

    let mut segment_lengths = Vec::with_capacity(n - 1);
    for i in 1..n {
        let dx = (vertices[i * 3] - vertices[(i - 1) * 3]) as f64;
        let dy = (vertices[i * 3 + 1] - vertices[(i - 1) * 3 + 1]) as f64;
        let dz = (vertices[i * 3 + 2] - vertices[(i - 1) * 3 + 2]) as f64;
        segment_lengths.push((dx * dx + dy * dy + dz * dz).sqrt());
    }

    let mut sorted = segment_lengths
        .iter()
        .copied()
        .filter(|d| d.is_finite())
        .collect::<Vec<f64>>();
    if sorted.is_empty() {
        return vec![false; n];
    }
    sorted.sort_by(|a, b| a.total_cmp(b));
    let median = sorted[sorted.len() / 2].max(1e-12);
    let threshold = median * factor.max(1.0);

    let mut breaks = vec![false; n];
    for i in 1..n {
        if segment_lengths[i - 1] > threshold || !segment_lengths[i - 1].is_finite() {
            breaks[i] = true;
        }
    }
    breaks
}

pub fn cusp_indices(vertices: &[f32], epsilon_scale: f64) -> Vec<u32> {
    let n = vertices.len() / 3;
    if n < 3 {
        return Vec::new();
    }

    let mut speeds = Vec::with_capacity(n.saturating_sub(2));
    for i in 1..(n - 1) {
        let vx = (vertices[(i + 1) * 3] - vertices[(i - 1) * 3]) as f64 * 0.5;
        let vy = (vertices[(i + 1) * 3 + 1] - vertices[(i - 1) * 3 + 1]) as f64 * 0.5;
        let vz = (vertices[(i + 1) * 3 + 2] - vertices[(i - 1) * 3 + 2]) as f64 * 0.5;
        let speed = vx.abs() + vy.abs() + vz.abs();
        if speed.is_finite() {
            speeds.push(speed);
        }
    }

    if speeds.is_empty() {
        return Vec::new();
    }

    speeds.sort_by(|a, b| a.total_cmp(b));
    let median = speeds[speeds.len() / 2].max(1e-12);
    let threshold = median * epsilon_scale.max(1e-6);

    let mut out = Vec::new();
    for i in 1..(n - 1) {
        let vx = (vertices[(i + 1) * 3] - vertices[(i - 1) * 3]) as f64 * 0.5;
        let vy = (vertices[(i + 1) * 3 + 1] - vertices[(i - 1) * 3 + 1]) as f64 * 0.5;
        let vz = (vertices[(i + 1) * 3 + 2] - vertices[(i - 1) * 3 + 2]) as f64 * 0.5;
        let speed = vx.abs() + vy.abs() + vz.abs();
        if speed.is_finite() && speed <= threshold {
            out.push(i as u32);
        }
    }
    out
}
