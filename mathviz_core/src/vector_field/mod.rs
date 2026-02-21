pub mod arrows;
pub mod differential;
pub mod streamlines;

use crate::{
    error::{MathvizError, MathvizResult},
    evaluator::ast::eval_ast,
    types::{AxisSpec, Point3, VectorFieldRequest, VectorFieldResponse},
};

pub fn process(request: VectorFieldRequest) -> MathvizResult<VectorFieldResponse> {
    let domain = request.domain.validate_and_clamp()?;
    let x_axis = domain
        .x
        .as_ref()
        .ok_or_else(|| MathvizError::DomainViolation("vector field requires x axis".to_string()))?;
    let y_axis = domain
        .y
        .as_ref()
        .ok_or_else(|| MathvizError::DomainViolation("vector field requires y axis".to_string()))?;
    let z_axis = domain
        .z
        .as_ref()
        .ok_or_else(|| MathvizError::DomainViolation("vector field requires z axis".to_string()))?;

    let field = eval_field(FieldEvalInput {
        x_axis,
        y_axis,
        z_axis,
        p_ast: &request.p_ast,
        q_ast: &request.q_ast,
        r_ast: &request.r_ast,
        params: &request.parameters,
        allow_non_finite: request.allow_non_finite,
    })?;

    let vectors: Vec<[f64; 3]> = field
        .p
        .iter()
        .zip(field.q.iter())
        .zip(field.r.iter())
        .map(|((px, qx), rx)| [*px, *qx, *rx])
        .collect();

    let unit_arrow = arrows::unit_arrow_geometry(if request.layer_id.is_empty() {
        "vector_field_unit_arrow".to_string()
    } else {
        format!("{}_unit_arrow", request.layer_id)
    });
    let instance_buffer = arrows::build_instances(&field.points, &vectors)?;

    let (divergence, curl) = if request.include_differentials {
        differential::compute_divergence_and_curl(&field.p, &field.q, &field.r, x_axis, y_axis, z_axis)
    } else {
        (Vec::new(), Vec::new())
    };

    let streamlines = if request.include_streamlines {
        let seeds = if request.streamline_seeds.is_empty() {
            default_seeds(x_axis, y_axis, z_axis)
        } else {
            request.streamline_seeds.clone()
        };

        streamlines::trace_streamlines(streamlines::StreamlineSpec {
            p_ast: &request.p_ast,
            q_ast: &request.q_ast,
            r_ast: &request.r_ast,
            domain,
            seeds: &seeds,
            params: &request.parameters,
            allow_non_finite: request.allow_non_finite,
            max_steps: request.streamline_max_steps.unwrap_or(400),
            h: request.streamline_step.unwrap_or(0.02),
        })?
    } else {
        Vec::new()
    };

    Ok(VectorFieldResponse {
        unit_arrow,
        instance_buffer,
        divergence,
        curl,
        streamlines,
    })
}

struct FieldEvalInput<'a> {
    x_axis: &'a AxisSpec,
    y_axis: &'a AxisSpec,
    z_axis: &'a AxisSpec,
    p_ast: &'a crate::types::ASTNode,
    q_ast: &'a crate::types::ASTNode,
    r_ast: &'a crate::types::ASTNode,
    params: &'a std::collections::BTreeMap<String, f64>,
    allow_non_finite: bool,
}

struct FieldEvalOutput {
    points: Vec<Point3>,
    p: Vec<f64>,
    q: Vec<f64>,
    r: Vec<f64>,
}

fn eval_field(input: FieldEvalInput<'_>) -> MathvizResult<FieldEvalOutput> {
    let n = input.x_axis.steps * input.y_axis.steps * input.z_axis.steps;
    let mut points = Vec::with_capacity(n);
    let mut p = Vec::with_capacity(n);
    let mut q = Vec::with_capacity(n);
    let mut r = Vec::with_capacity(n);

    for iz in 0..input.z_axis.steps {
        let z = input.z_axis.value_at(iz);
        for iy in 0..input.y_axis.steps {
            let y = input.y_axis.value_at(iy);
            for ix in 0..input.x_axis.steps {
                let x = input.x_axis.value_at(ix);
                let mut bindings: [(&str, f64); 16] = [("", 0.0); 16];
                let mut len = 0usize;
                bindings[len] = ("x", x);
                len += 1;
                bindings[len] = ("y", y);
                len += 1;
                bindings[len] = ("z", z);
                len += 1;
                bindings[len] = ("t", 0.0);
                len += 1;

                for (name, value) in input.params {
                    if len < bindings.len() {
                        bindings[len] = (name.as_str(), *value);
                        len += 1;
                    }
                }

                points.push(Point3 { x, y, z });
                p.push(eval_ast(input.p_ast, &bindings[..len], input.allow_non_finite)?);
                q.push(eval_ast(input.q_ast, &bindings[..len], input.allow_non_finite)?);
                r.push(eval_ast(input.r_ast, &bindings[..len], input.allow_non_finite)?);
            }
        }
    }

    Ok(FieldEvalOutput { points, p, q, r })
}

fn default_seeds(x: &AxisSpec, y: &AxisSpec, z: &AxisSpec) -> Vec<Point3> {
    let mx = (x.min + x.max) * 0.5;
    let my = (y.min + y.max) * 0.5;
    let mz = (z.min + z.max) * 0.5;
    vec![
        Point3 {
            x: mx,
            y: my,
            z: mz,
        },
        Point3 {
            x: x.min,
            y: my,
            z: mz,
        },
        Point3 {
            x: x.max,
            y: my,
            z: mz,
        },
        Point3 {
            x: mx,
            y: y.min,
            z: mz,
        },
        Point3 {
            x: mx,
            y: y.max,
            z: mz,
        },
    ]
}
