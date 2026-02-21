use crate::{
    error::{MathvizError, MathvizResult},
    evaluator::ast::eval_ast,
    types::{GeometryBuffer, PartitionMethod, RiemannRequest},
};

pub fn generate(request: RiemannRequest) -> MathvizResult<GeometryBuffer> {
    let domain = request.domain.validate_and_clamp()?;
    let x_axis = domain
        .x
        .as_ref()
        .ok_or_else(|| MathvizError::DomainViolation("riemann sum requires x axis".to_string()))?;

    let n = request.subdivisions.max(1);
    let from = request.from_index.unwrap_or(0).min(n);
    let method = request.method.unwrap_or(PartitionMethod::Midpoint);
    let dx = (x_axis.max - x_axis.min) / n as f64;

    let count = n - from;
    let mut vertices = Vec::with_capacity(count * 4 * 3);
    let mut normals = Vec::with_capacity(count * 4 * 3);
    let mut indices = Vec::with_capacity(count * 6);

    for i in from..n {
        let x0 = x_axis.min + i as f64 * dx;
        let x1 = x0 + dx;
        let x_rep = match method {
            PartitionMethod::Left => x0,
            PartitionMethod::Right => x1,
            PartitionMethod::Midpoint => (x0 + x1) * 0.5,
            PartitionMethod::Trapezoid => x0,
        };

        let f0 = eval_scalar(
            &request.ast,
            x0,
            &request.parameters,
            request.allow_non_finite,
        )?;
        let f1 = eval_scalar(
            &request.ast,
            x1,
            &request.parameters,
            request.allow_non_finite,
        )?;

        let f = match method {
            PartitionMethod::Trapezoid => 0.5 * (f0 + f1),
            _ => {
                if matches!(
                    method,
                    PartitionMethod::Left | PartitionMethod::Right | PartitionMethod::Midpoint
                ) {
                    eval_scalar(
                        &request.ast,
                        x_rep,
                        &request.parameters,
                        request.allow_non_finite,
                    )?
                } else {
                    f0
                }
            }
        };

        let base = (vertices.len() / 3) as u32;

        vertices.extend_from_slice(&[
            x0 as f32, 0.0, 0.0, x1 as f32, 0.0, 0.0, x1 as f32, f as f32, 0.0, x0 as f32,
            f as f32, 0.0,
        ]);

        normals.extend_from_slice(&[0.0, 0.0, 1.0, 0.0, 0.0, 1.0, 0.0, 0.0, 1.0, 0.0, 0.0, 1.0]);

        indices.extend_from_slice(&[base, base + 1, base + 2, base, base + 2, base + 3]);
    }

    Ok(GeometryBuffer {
        vertex_buffer: vertices,
        normal_buffer: normals,
        index_buffer: indices,
        uv_buffer: Vec::new(),
        layer_id: if request.layer_id.is_empty() {
            "riemann".to_string()
        } else {
            request.layer_id
        },
        is_delta: request.from_index.unwrap_or(0) > 0,
    })
}

fn eval_scalar(
    ast: &crate::types::ASTNode,
    x: f64,
    params: &std::collections::BTreeMap<String, f64>,
    allow_non_finite: bool,
) -> MathvizResult<f64> {
    let mut bindings: [(&str, f64); 16] = [("", 0.0); 16];
    let mut len = 0usize;
    bindings[len] = ("x", x);
    len += 1;
    for (name, value) in params {
        if len < bindings.len() {
            bindings[len] = (name.as_str(), *value);
            len += 1;
        }
    }
    eval_ast(ast, &bindings[..len], allow_non_finite)
}
