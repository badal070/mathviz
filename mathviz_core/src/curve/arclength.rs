pub fn normalized_arc_length(vertices: &[f32]) -> Vec<f32> {
    let n = vertices.len() / 3;
    if n == 0 {
        return Vec::new();
    }

    let mut arc = vec![0.0f32; n];
    let mut total = 0.0f64;
    for (i, arc_i) in arc.iter_mut().enumerate().skip(1) {
        let p0 = i - 1;
        let dx = vertices[i * 3] as f64 - vertices[p0 * 3] as f64;
        let dy = vertices[i * 3 + 1] as f64 - vertices[p0 * 3 + 1] as f64;
        let dz = vertices[i * 3 + 2] as f64 - vertices[p0 * 3 + 2] as f64;
        total += (dx * dx + dy * dy + dz * dz).sqrt();
        *arc_i = total as f32;
    }

    if total > 0.0 {
        let inv = (1.0 / total) as f32;
        for value in &mut arc {
            *value *= inv;
        }
    }

    arc
}
