use crate::{
    curve::{
        arclength::normalized_arc_length,
        discontinuity::{cusp_indices, discontinuity_breaks},
    },
    error::{MathvizError, MathvizResult},
    evaluator::ast::eval_ast,
    types::{ASTNode, AxisSpec, CurveTraceRequest, CurveTraceResponse, GeometryBuffer},
};

pub fn trace_explicit_curve(request: CurveTraceRequest) -> MathvizResult<CurveTraceResponse> {
    if request.x_ast.is_some() || request.y_ast.is_some() || request.z_ast.is_some() {
        return trace_parametric_curve(request);
    }

    let threshold = request.threshold_factor();
    let domain = request.domain.validate_and_clamp()?;
    let x_axis = domain
        .x
        .as_ref()
        .ok_or_else(|| MathvizError::DomainViolation("curve trace requires x axis".to_string()))?;

    let vertices = eval_curve_vertices(
        &request.ast,
        x_axis,
        &request.parameters,
        request.allow_non_finite,
    )?;
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
        layer_id: default_curve_layer_id(&request.layer_id),
        is_delta: false,
    };

    Ok(CurveTraceResponse {
        geometry,
        arc_length: arc,
        cusp_indices: Vec::new(),
    })
}

fn trace_parametric_curve(request: CurveTraceRequest) -> MathvizResult<CurveTraceResponse> {
    let threshold = request.threshold_factor();
    let domain = request.domain.validate_and_clamp()?;
    let t_axis = domain.t.as_ref().or(domain.x.as_ref()).ok_or_else(|| {
        MathvizError::DomainViolation("parametric curve requires t or x axis".to_string())
    })?;
    let x_ast = request.x_ast.as_ref().ok_or_else(|| {
        MathvizError::DomainViolation("parametric curve requires x_ast".to_string())
    })?;
    let y_ast = request.y_ast.as_ref().ok_or_else(|| {
        MathvizError::DomainViolation("parametric curve requires y_ast".to_string())
    })?;
    let z_ast = request.z_ast.as_ref();
    let param_name = request
        .parameter_name
        .as_deref()
        .filter(|name| !name.trim().is_empty())
        .unwrap_or("t");

    let mut vertices = Vec::with_capacity(t_axis.steps * 3);
    for i in 0..t_axis.steps {
        let t_value = t_axis.value_at(i);

        let mut stack_bindings: [(&str, f64); 16] = [("", 0.0); 16];
        let mut len = 0usize;
        stack_bindings[len] = (param_name, t_value);
        len += 1;
        if param_name != "t" {
            stack_bindings[len] = ("t", t_value);
            len += 1;
        }

        for (name, value) in &request.parameters {
            if len < stack_bindings.len() {
                stack_bindings[len] = (name.as_str(), *value);
                len += 1;
            }
        }

        let x = eval_ast(x_ast, &stack_bindings[..len], request.allow_non_finite)?;
        let y = eval_ast(y_ast, &stack_bindings[..len], request.allow_non_finite)?;
        let z = match z_ast {
            Some(node) => eval_ast(node, &stack_bindings[..len], request.allow_non_finite)?,
            None => 0.0,
        };
        vertices.extend_from_slice(&[x as f32, y as f32, z as f32]);
    }

    let breaks = discontinuity_breaks(&vertices, threshold);
    let mut indices = Vec::with_capacity(t_axis.steps + t_axis.steps / 8);
    for i in 0..t_axis.steps as u32 {
        if i > 0 && breaks[i as usize] {
            indices.push(u32::MAX);
        }
        indices.push(i);
    }

    let arc = normalized_arc_length(&vertices);
    let cusps = cusp_indices(&vertices, 1e-3);
    let geometry = GeometryBuffer {
        vertex_buffer: vertices,
        normal_buffer: Vec::new(),
        index_buffer: indices,
        uv_buffer: Vec::new(),
        layer_id: default_curve_layer_id(&request.layer_id),
        is_delta: false,
    };

    Ok(CurveTraceResponse {
        geometry,
        arc_length: arc,
        cusp_indices: cusps,
    })
}

fn default_curve_layer_id(layer_id: &str) -> String {
    if layer_id.is_empty() {
        "curve".to_string()
    } else {
        layer_id.to_string()
    }
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
