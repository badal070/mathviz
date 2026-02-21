pub mod rk4;
pub mod rk45;

use std::collections::BTreeMap;

use rayon::prelude::*;

use crate::{
    error::{MathvizError, MathvizResult},
    evaluator::ast::eval_ast,
    types::{DomainSpec, IVPSpec, OdeBatchRequest, OdeMethod, TrajectoryBuffer},
};

pub fn solve_batch(request: OdeBatchRequest) -> MathvizResult<Vec<TrajectoryBuffer>> {
    if request.ivps.is_empty() {
        return Err(MathvizError::OdeError(
            "IVP list must not be empty".to_string(),
        ));
    }

    request
        .ivps
        .par_iter()
        .map(|ivp| solve_single_ivp(ivp, &request.parameters, request.allow_non_finite))
        .collect()
}

pub fn solve_single_ivp(
    ivp: &IVPSpec,
    params: &BTreeMap<String, f64>,
    allow_non_finite: bool,
) -> MathvizResult<TrajectoryBuffer> {
    validate_ivp(ivp)?;

    let method = ivp.method.clone().unwrap_or(OdeMethod::Rk45);
    match method {
        OdeMethod::Rk4 => rk4::integrate(ivp, params, allow_non_finite),
        OdeMethod::Rk45 => rk45::integrate(ivp, params, allow_non_finite),
    }
}

pub(crate) fn validate_ivp(ivp: &IVPSpec) -> MathvizResult<()> {
    let dim = ivp.initial_state.len();
    if dim == 0 {
        return Err(MathvizError::OdeError(
            "initial_state must not be empty".to_string(),
        ));
    }
    if dim > 3 {
        return Err(MathvizError::OdeError(
            "state dimension > 3 is not supported for visualization".to_string(),
        ));
    }
    if ivp.derivatives.len() != dim {
        return Err(MathvizError::OdeError(format!(
            "derivative count ({}) does not match state dimension ({dim})",
            ivp.derivatives.len()
        )));
    }
    if ivp.t0.partial_cmp(&ivp.t_end) != Some(std::cmp::Ordering::Less) {
        return Err(MathvizError::OdeError(
            "t0 must be strictly less than t_end".to_string(),
        ));
    }
    if let Some(h) = ivp.step_size {
        if h <= 0.0 {
            return Err(MathvizError::OdeError("step_size must be > 0".to_string()));
        }
    }
    if let Some(max_steps) = ivp.max_steps {
        if max_steps == 0 {
            return Err(MathvizError::OdeError("max_steps must be > 0".to_string()));
        }
    }

    if let Some(domain) = &ivp.domain {
        let validated = domain.clone().validate_and_clamp()?;
        let x_ok = validated.x.is_none() || dim >= 1;
        let y_ok = validated.y.is_none() || dim >= 2;
        let z_ok = validated.z.is_none() || dim >= 3;
        if !(x_ok && y_ok && z_ok) {
            return Err(MathvizError::OdeError(
                "domain axes exceed state dimension".to_string(),
            ));
        }
    }

    Ok(())
}

pub(crate) fn eval_derivative(
    ivp: &IVPSpec,
    state: &[f64],
    t: f64,
    params: &BTreeMap<String, f64>,
    allow_non_finite: bool,
    out: &mut [f64],
) -> MathvizResult<()> {
    let dim = state.len();
    if out.len() != dim {
        return Err(MathvizError::OdeError(
            "derivative output buffer dimension mismatch".to_string(),
        ));
    }

    // Keep the common path allocation-free for small parameter maps.
    let mut stack_bindings: [(&str, f64); 16] = [("", 0.0); 16];
    let mut len = 0usize;

    if dim >= 1 {
        stack_bindings[len] = ("x", state[0]);
        len += 1;
    }
    if dim >= 2 {
        stack_bindings[len] = ("y", state[1]);
        len += 1;
    }
    if dim >= 3 {
        stack_bindings[len] = ("z", state[2]);
        len += 1;
    }
    stack_bindings[len] = ("t", t);
    len += 1;

    for (name, value) in params {
        if len < stack_bindings.len() {
            stack_bindings[len] = (name.as_str(), *value);
            len += 1;
        } else {
            return eval_derivative_heap(ivp, state, t, params, allow_non_finite, out);
        }
    }

    for (i, deriv_ast) in ivp.derivatives.iter().enumerate() {
        out[i] = eval_ast(deriv_ast, &stack_bindings[..len], allow_non_finite)?;
    }
    Ok(())
}

fn eval_derivative_heap(
    ivp: &IVPSpec,
    state: &[f64],
    t: f64,
    params: &BTreeMap<String, f64>,
    allow_non_finite: bool,
    out: &mut [f64],
) -> MathvizResult<()> {
    let mut bindings = Vec::with_capacity(4 + params.len());
    if !state.is_empty() {
        bindings.push(("x", state[0]));
    }
    if state.len() >= 2 {
        bindings.push(("y", state[1]));
    }
    if state.len() >= 3 {
        bindings.push(("z", state[2]));
    }
    bindings.push(("t", t));
    for (name, value) in params {
        bindings.push((name.as_str(), *value));
    }

    for (i, deriv_ast) in ivp.derivatives.iter().enumerate() {
        out[i] = eval_ast(deriv_ast, &bindings, allow_non_finite)?;
    }
    Ok(())
}

pub(crate) fn check_state_in_domain(state: &[f64], t: f64, domain: &Option<DomainSpec>) -> bool {
    let Some(domain) = domain else {
        return true;
    };

    let x_ok = match (&domain.x, state.first()) {
        (Some(axis), Some(x)) => *x >= axis.min && *x <= axis.max,
        (None, _) => true,
        _ => false,
    };
    let y_ok = match (&domain.y, state.get(1)) {
        (Some(axis), Some(y)) => *y >= axis.min && *y <= axis.max,
        (None, _) => true,
        _ => false,
    };
    let z_ok = match (&domain.z, state.get(2)) {
        (Some(axis), Some(z)) => *z >= axis.min && *z <= axis.max,
        (None, _) => true,
        _ => false,
    };
    let t_ok = match &domain.t {
        Some(axis) => t >= axis.min && t <= axis.max,
        None => true,
    };

    x_ok && y_ok && z_ok && t_ok
}

pub(crate) fn pack_state_triples(state: &[f64], out: &mut Vec<f64>) {
    out.push(*state.first().unwrap_or(&0.0));
    out.push(*state.get(1).unwrap_or(&0.0));
    out.push(*state.get(2).unwrap_or(&0.0));
}
