use std::collections::BTreeMap;

use crate::{
    error::{MathvizError, MathvizResult},
    ode::{check_state_in_domain, eval_derivative, pack_state_triples},
    types::{IVPSpec, TrajectoryBuffer},
};

pub fn integrate(
    ivp: &IVPSpec,
    params: &BTreeMap<String, f64>,
    allow_non_finite: bool,
) -> MathvizResult<TrajectoryBuffer> {
    let dim = ivp.initial_state.len();
    let max_steps = ivp.max_steps.unwrap_or(50_000);

    let mut h = ivp
        .step_size
        .unwrap_or_else(|| (ivp.t_end - ivp.t0) / 1000.0)
        .abs();
    if h <= 0.0 {
        return Err(MathvizError::OdeError(
            "RK4 step size must be > 0".to_string(),
        ));
    }

    let mut t = ivp.t0;
    let mut state = ivp.initial_state.clone();

    if !check_state_in_domain(&state, t, &ivp.domain) {
        return Err(MathvizError::OdeError(
            "initial state is outside domain".to_string(),
        ));
    }

    let mut times = Vec::with_capacity(max_steps.saturating_add(1));
    let mut packed_state = Vec::with_capacity(max_steps.saturating_add(1) * 3);
    times.push(t);
    pack_state_triples(&state, &mut packed_state);

    let mut k1 = vec![0.0; dim];
    let mut k2 = vec![0.0; dim];
    let mut k3 = vec![0.0; dim];
    let mut k4 = vec![0.0; dim];
    let mut tmp = vec![0.0; dim];

    let mut steps_taken = 0usize;
    while t < ivp.t_end {
        if steps_taken >= max_steps {
            return Err(MathvizError::OdeError(
                "RK4 step limit exceeded".to_string(),
            ));
        }

        let remaining = ivp.t_end - t;
        if remaining < h {
            h = remaining;
        }

        eval_derivative(ivp, &state, t, params, allow_non_finite, &mut k1)?;

        for i in 0..dim {
            tmp[i] = state[i] + 0.5 * h * k1[i];
        }
        eval_derivative(ivp, &tmp, t + 0.5 * h, params, allow_non_finite, &mut k2)?;

        for i in 0..dim {
            tmp[i] = state[i] + 0.5 * h * k2[i];
        }
        eval_derivative(ivp, &tmp, t + 0.5 * h, params, allow_non_finite, &mut k3)?;

        for i in 0..dim {
            tmp[i] = state[i] + h * k3[i];
        }
        eval_derivative(ivp, &tmp, t + h, params, allow_non_finite, &mut k4)?;

        for i in 0..dim {
            state[i] += (h / 6.0) * (k1[i] + 2.0 * k2[i] + 2.0 * k3[i] + k4[i]);
            if !allow_non_finite && !state[i].is_finite() {
                return Err(MathvizError::OdeError(
                    "RK4 integration produced non-finite state".to_string(),
                ));
            }
        }

        t += h;
        steps_taken += 1;
        times.push(t);
        pack_state_triples(&state, &mut packed_state);

        if !check_state_in_domain(&state, t, &ivp.domain) {
            return Ok(TrajectoryBuffer {
                state: packed_state,
                times,
                dimension: dim,
                layer_id: if ivp.layer_id.is_empty() {
                    "rk4".to_string()
                } else {
                    ivp.layer_id.clone()
                },
                terminated_reason: "domain_exit".to_string(),
            });
        }
    }

    Ok(TrajectoryBuffer {
        state: packed_state,
        times,
        dimension: dim,
        layer_id: if ivp.layer_id.is_empty() {
            "rk4".to_string()
        } else {
            ivp.layer_id.clone()
        },
        terminated_reason: "completed".to_string(),
    })
}
