pub mod ast;
pub mod constants;

use std::collections::{BTreeMap, HashMap, HashSet};

use rayon::prelude::*;

use crate::{
    curve::tracer::eval_curve_vertices,
    error::{MathvizError, MathvizResult},
    evaluator::ast::eval_ast,
    mesh::explicit::build_explicit_surface,
    mesh::implicit::marching_cubes,
    types::{BatchEntry, BatchOutcome, BatchRequest, BatchResult, ConceptTypeHint, GeometryBuffer},
};

pub fn evaluate_batch(request: BatchRequest) -> BatchResult {
    let BatchRequest {
        entries,
        parameters,
        allow_non_finite,
        render_bound,
    } = request;

    let unique_entries = dedupe_entries(entries);
    let ctx = EvalContext {
        parameters: &parameters,
        allow_non_finite,
        render_bound,
    };

    let computed: HashMap<String, Result<GeometryBuffer, MathvizError>> = unique_entries
        .par_iter()
        .map(|entry| (entry.hash_key.clone(), evaluate_entry(entry, &ctx)))
        .collect();

    let mut out = BTreeMap::new();
    for (hash, result) in computed {
        match result {
            Ok(geometry) => {
                out.insert(
                    hash,
                    BatchOutcome {
                        ok: Some(geometry),
                        err: None,
                    },
                );
            }
            Err(err) => {
                out.insert(
                    hash,
                    BatchOutcome {
                        ok: None,
                        err: Some(err.to_string()),
                    },
                );
            }
        }
    }

    out
}

fn dedupe_entries(entries: Vec<BatchEntry>) -> Vec<BatchEntry> {
    let mut seen = HashSet::new();
    let mut out = Vec::new();
    for entry in entries {
        if seen.insert(entry.hash_key.clone()) {
            out.push(entry);
        }
    }
    out
}

struct EvalContext<'a> {
    parameters: &'a std::collections::BTreeMap<String, f64>,
    allow_non_finite: bool,
    render_bound: f64,
}

fn evaluate_entry(entry: &BatchEntry, ctx: &EvalContext<'_>) -> MathvizResult<GeometryBuffer> {
    let domain = entry.domain.clone().validate_and_clamp()?;
    match entry.concept_type {
        ConceptTypeHint::ExplicitSurface => {
            let x_axis = domain.x.as_ref().ok_or_else(|| {
                MathvizError::DomainViolation("explicit surface requires x axis".to_string())
            })?;
            let y_axis = domain.y.as_ref().ok_or_else(|| {
                MathvizError::DomainViolation("explicit surface requires y axis".to_string())
            })?;

            let z_field = eval_surface_field(
                &entry.ast,
                x_axis,
                y_axis,
                ctx.parameters,
                ctx.allow_non_finite,
            )?;

            build_explicit_surface(
                x_axis,
                y_axis,
                &z_field,
                ctx.render_bound,
                layer_name(entry),
            )
        }
        ConceptTypeHint::Curve2d => {
            let x_axis = domain.x.as_ref().ok_or_else(|| {
                MathvizError::DomainViolation("curve requires x axis".to_string())
            })?;
            let vertices =
                eval_curve_vertices(&entry.ast, x_axis, ctx.parameters, ctx.allow_non_finite)?;
            let indices = (0..x_axis.steps as u32).collect::<Vec<u32>>();
            Ok(GeometryBuffer {
                vertex_buffer: vertices,
                normal_buffer: Vec::new(),
                index_buffer: indices,
                uv_buffer: Vec::new(),
                layer_id: layer_name(entry),
                is_delta: false,
            })
        }
        ConceptTypeHint::ImplicitSurface => {
            let x_axis = domain.x.as_ref().ok_or_else(|| {
                MathvizError::DomainViolation("implicit surface requires x axis".to_string())
            })?;
            let y_axis = domain.y.as_ref().ok_or_else(|| {
                MathvizError::DomainViolation("implicit surface requires y axis".to_string())
            })?;
            let z_axis = domain.z.as_ref().ok_or_else(|| {
                MathvizError::DomainViolation("implicit surface requires z axis".to_string())
            })?;
            let field = eval_volume_field(
                &entry.ast,
                x_axis,
                y_axis,
                z_axis,
                ctx.parameters,
                ctx.allow_non_finite,
            )?;
            marching_cubes(x_axis, y_axis, z_axis, &field, layer_name(entry))
        }
        _ => Err(MathvizError::UnsupportedOperation(format!(
            "concept type {:?} is not implemented yet",
            entry.concept_type
        ))),
    }
}

fn layer_name(entry: &BatchEntry) -> String {
    if entry.layer_id.is_empty() {
        entry.hash_key.clone()
    } else {
        entry.layer_id.clone()
    }
}

fn eval_surface_field(
    ast: &crate::types::ASTNode,
    x_axis: &crate::types::AxisSpec,
    y_axis: &crate::types::AxisSpec,
    params: &std::collections::BTreeMap<String, f64>,
    allow_non_finite: bool,
) -> MathvizResult<Vec<f64>> {
    let mut out = vec![0.0f64; x_axis.steps * y_axis.steps];

    out.par_chunks_mut(x_axis.steps).enumerate().try_for_each(
        |(iy, row)| -> MathvizResult<()> {
            let y = y_axis.value_at(iy);
            for (ix, value) in row.iter_mut().enumerate() {
                let x = x_axis.value_at(ix);
                let mut stack_bindings: [(&str, f64); 8] = [("", 0.0); 8];
                let mut len = 0usize;

                stack_bindings[len] = ("x", x);
                len += 1;
                stack_bindings[len] = ("y", y);
                len += 1;

                for (name, param) in params {
                    if len < stack_bindings.len() {
                        stack_bindings[len] = (name.as_str(), *param);
                        len += 1;
                    }
                }

                *value = eval_ast(ast, &stack_bindings[..len], allow_non_finite)?;
            }
            Ok(())
        },
    )?;

    Ok(out)
}

fn eval_volume_field(
    ast: &crate::types::ASTNode,
    x_axis: &crate::types::AxisSpec,
    y_axis: &crate::types::AxisSpec,
    z_axis: &crate::types::AxisSpec,
    params: &std::collections::BTreeMap<String, f64>,
    allow_non_finite: bool,
) -> MathvizResult<Vec<f64>> {
    let mut out = vec![0.0f64; x_axis.steps * y_axis.steps * z_axis.steps];

    out.par_chunks_mut(x_axis.steps * y_axis.steps)
        .enumerate()
        .try_for_each(|(iz, slice)| -> MathvizResult<()> {
            let z = z_axis.value_at(iz);
            for iy in 0..y_axis.steps {
                let y = y_axis.value_at(iy);
                let row = &mut slice[iy * x_axis.steps..(iy + 1) * x_axis.steps];
                for (ix, value) in row.iter_mut().enumerate() {
                    let x = x_axis.value_at(ix);
                    let mut bindings: [(&str, f64); 16] = [("", 0.0); 16];
                    let mut len = 0usize;
                    bindings[len] = ("x", x);
                    len += 1;
                    bindings[len] = ("y", y);
                    len += 1;
                    bindings[len] = ("z", z);
                    len += 1;

                    for (name, param) in params {
                        if len < bindings.len() {
                            bindings[len] = (name.as_str(), *param);
                            len += 1;
                        }
                    }

                    *value = eval_ast(ast, &bindings[..len], allow_non_finite)?;
                }
            }
            Ok(())
        })?;

    Ok(out)
}
