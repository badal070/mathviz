use crate::{
    curve::{arclength::normalized_arc_length, discontinuity::discontinuity_breaks},
    error::{MathvizError, MathvizResult},
    evaluator::ast::eval_ast,
    types::{AxisSpec, ASTNode, CurveTraceRequest, CurveTraceResponse, GeometryBuffer},
};

pub fn trace_explicit_curve(request: CurveTraceRequest) -> MathvizResult<CurveTraceResponse> {
    let threshold = request.threshold_factor();
    let domain = request.domain.validate_and_clamp()?;
    let x_axis = domain
        .x
        .as_ref()
        .ok_or_else(|| MathvizError::DomainViolation("curve trace requires x axis".to_string()))?;

    let vertices = eval_curve_vertices(&request.ast, x_axis, &request.parameters, false)?;
    let breaks = discontinuity_breaks(&vertices, threshold);

    let mut indices = Vec::with_capacity(x_axis.steps + x_axis.steps / 8);
    for i in 0..x_axis.steps as u32 {
        if i > 0 && breaks[i as usize] {
            indices.push(u32::MAX);
        }
        indices.push(i);
    }

    let arc = normalized_arc_length(&vertices);
    let geometry = GeometryBuffer {
        vertex_buffer: vertices,
        normal_buffer: Vec::new(),
        index_buffer: indices,
        uv_buffer: Vec::new(),
        layer_id: "curve".to_string(),
        is_delta: false,
    };

    Ok(CurveTraceResponse {
        geometry,
        arc_length: arc,
    })
}

pub fn eval_curve_vertices(
    ast: &ASTNode,
    x_axis: &AxisSpec,
    params: &std::collections::BTreeMap<String, f64>,
    allow_non_finite: bool,
) -> MathvizResult<Vec<f32>> {
    let mut vertices = Vec::with_capacity(x_axis.steps * 3);
    for i in 0..x_axis.steps {
        let x = x_axis.value_at(i);
        let mut stack_bindings: [(&str, f64); 8] = [("", 0.0); 8];
        let mut len = 0usize;

        stack_bindings[len] = ("x", x);
        len += 1;

        for (name, value) in params {
            if len < stack_bindings.len() {
                stack_bindings[len] = (name.as_str(), *value);
                len += 1;
            }
        }

        let y = eval_ast(ast, &stack_bindings[..len], allow_non_finite)?;
        vertices.push(x as f32);
        vertices.push(y as f32);
        vertices.push(0.0);
    }

    Ok(vertices)
}
