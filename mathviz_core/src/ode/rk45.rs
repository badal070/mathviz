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
    let interval = ivp.t_end - ivp.t0;
    let max_steps = ivp.max_steps.unwrap_or(100_000);

    let abs_tol = ivp.abs_tol.unwrap_or(1e-7).abs().max(1e-15);
    let rel_tol = ivp.rel_tol.unwrap_or(1e-6).abs().max(1e-15);
    let h_min = ivp.h_min.unwrap_or(1e-9).abs().max(1e-15);
    let mut h_max = ivp.h_max.unwrap_or(interval / 10.0).abs().max(h_min * 10.0);

    let mut h = ivp
        .step_size
        .unwrap_or(interval / 200.0)
        .abs()
        .clamp(h_min, h_max);
    if h <= 0.0 {
        return Err(MathvizError::OdeError(
            "RK45 initial step size must be > 0".to_string(),
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
    let mut k5 = vec![0.0; dim];
    let mut k6 = vec![0.0; dim];
    let mut k7 = vec![0.0; dim];

    let mut tmp = vec![0.0; dim];
    let mut y4 = vec![0.0; dim];
    let mut y5 = vec![0.0; dim];

    let mut accepted_steps = 0usize;
    let mut attempts = 0usize;

    while t < ivp.t_end {
        attempts += 1;
        if attempts > max_steps {
            return Err(MathvizError::OdeError(
                "RK45 step limit exceeded".to_string(),
            ));
        }

        let remaining = ivp.t_end - t;
        if h > remaining {
            h = remaining;
        }

        eval_derivative(ivp, &state, t, params, allow_non_finite, &mut k1)?;

        for i in 0..dim {
            tmp[i] = state[i] + h * (1.0 / 5.0) * k1[i];
        }
        eval_derivative(ivp, &tmp, t + h * (1.0 / 5.0), params, allow_non_finite, &mut k2)?;

        for i in 0..dim {
            tmp[i] = state[i] + h * ((3.0 / 40.0) * k1[i] + (9.0 / 40.0) * k2[i]);
        }
        eval_derivative(ivp, &tmp, t + h * (3.0 / 10.0), params, allow_non_finite, &mut k3)?;

        for i in 0..dim {
            tmp[i] = state[i]
                + h * ((44.0 / 45.0) * k1[i] - (56.0 / 15.0) * k2[i] + (32.0 / 9.0) * k3[i]);
        }
        eval_derivative(ivp, &tmp, t + h * (4.0 / 5.0), params, allow_non_finite, &mut k4)?;

        for i in 0..dim {
            tmp[i] = state[i]
                + h * ((19372.0 / 6561.0) * k1[i]
                    - (25360.0 / 2187.0) * k2[i]
                    + (64448.0 / 6561.0) * k3[i]
                    - (212.0 / 729.0) * k4[i]);
        }
        eval_derivative(ivp, &tmp, t + h * (8.0 / 9.0), params, allow_non_finite, &mut k5)?;

        for i in 0..dim {
            tmp[i] = state[i]
                + h * ((9017.0 / 3168.0) * k1[i]
                    - (355.0 / 33.0) * k2[i]
                    + (46732.0 / 5247.0) * k3[i]
                    + (49.0 / 176.0) * k4[i]
                    - (5103.0 / 18656.0) * k5[i]);
        }
        eval_derivative(ivp, &tmp, t + h, params, allow_non_finite, &mut k6)?;

        for i in 0..dim {
            y5[i] = state[i]
                + h * ((35.0 / 384.0) * k1[i]
                    + (500.0 / 1113.0) * k3[i]
                    + (125.0 / 192.0) * k4[i]
                    - (2187.0 / 6784.0) * k5[i]
                    + (11.0 / 84.0) * k6[i]);
        }

        eval_derivative(ivp, &y5, t + h, params, allow_non_finite, &mut k7)?;

        for i in 0..dim {
            y4[i] = state[i]
                + h * ((5179.0 / 57600.0) * k1[i]
                    + (7571.0 / 16695.0) * k3[i]
                    + (393.0 / 640.0) * k4[i]
                    - (92097.0 / 339200.0) * k5[i]
                    + (187.0 / 2100.0) * k6[i]
                    + (1.0 / 40.0) * k7[i]);
        }

        let mut err_norm = 0.0f64;
        for i in 0..dim {
            let sc = abs_tol + rel_tol * state[i].abs().max(y5[i].abs());
            let e = (y5[i] - y4[i]).abs() / sc.max(1e-15);
            if e > err_norm {
                err_norm = e;
            }
        }

        let accepted = err_norm <= 1.0;

        if accepted {
            t += h;
            accepted_steps += 1;
            state.copy_from_slice(&y5);
            for value in &state {
                if !allow_non_finite && !value.is_finite() {
                    return Err(MathvizError::OdeError(
                        "RK45 integration produced non-finite state".to_string(),
                    ));
                }
            }
            times.push(t);
            pack_state_triples(&state, &mut packed_state);

            if !check_state_in_domain(&state, t, &ivp.domain) {
                return Ok(TrajectoryBuffer {
                    state: packed_state,
                    times,
                    dimension: dim,
                    layer_id: if ivp.layer_id.is_empty() {
                        "rk45".to_string()
                    } else {
                        ivp.layer_id.clone()
                    },
                    terminated_reason: "domain_exit".to_string(),
                });
            }

            if accepted_steps >= max_steps {
                return Err(MathvizError::OdeError(
                    "RK45 accepted-step limit exceeded".to_string(),
                ));
            }
        }

        let factor = if err_norm == 0.0 {
            5.0
        } else {
            (0.9 * err_norm.powf(-0.2)).clamp(0.2, 5.0)
        };
        h = (h * factor).clamp(h_min, h_max);

        h_max = h_max.max(h_min * 10.0);
        if h <= h_min && !accepted {
            return Err(MathvizError::OdeError(
                "RK45 step size fell below h_min in stiff region".to_string(),
            ));
        }
    }

    Ok(TrajectoryBuffer {
        state: packed_state,
        times,
        dimension: dim,
        layer_id: if ivp.layer_id.is_empty() {
            "rk45".to_string()
        } else {
            ivp.layer_id.clone()
        },
        terminated_reason: "completed".to_string(),
    })
}
