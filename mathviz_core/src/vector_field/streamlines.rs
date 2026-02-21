use std::collections::BTreeMap;

use crate::{
    error::MathvizResult,
    ode,
    types::{ASTNode, DomainSpec, GeometryBuffer, IVPSpec, OdeMethod, Point3},
};

pub struct StreamlineSpec<'a> {
    pub p_ast: &'a ASTNode,
    pub q_ast: &'a ASTNode,
    pub r_ast: &'a ASTNode,
    pub domain: DomainSpec,
    pub seeds: &'a [Point3],
    pub params: &'a BTreeMap<String, f64>,
    pub allow_non_finite: bool,
    pub max_steps: usize,
    pub h: f64,
}

pub fn trace_streamlines(spec: StreamlineSpec<'_>) -> MathvizResult<Vec<GeometryBuffer>> {
    let mut out = Vec::with_capacity(spec.seeds.len());

    for (i, seed) in spec.seeds.iter().enumerate() {
        let ivp = IVPSpec {
            derivatives: vec![spec.p_ast.clone(), spec.q_ast.clone(), spec.r_ast.clone()],
            initial_state: vec![seed.x, seed.y, seed.z],
            t0: 0.0,
            t_end: spec.h * spec.max_steps as f64,
            method: Some(OdeMethod::Rk45),
            step_size: Some(spec.h),
            max_steps: Some(spec.max_steps),
            abs_tol: Some(1e-7),
            rel_tol: Some(1e-6),
            h_min: Some((spec.h * 1e-3).max(1e-10)),
            h_max: Some(spec.h * 4.0),
            domain: Some(spec.domain.clone()),
            layer_id: format!("streamline_{i}"),
        };

        let traj = ode::solve_single_ivp(&ivp, spec.params, spec.allow_non_finite)?;
        let count = traj.state.len() / 3;
        let mut vertices = Vec::with_capacity(count * 3);
        let mut indices = Vec::with_capacity(count);

        for idx in 0..count {
            vertices.push(traj.state[idx * 3] as f32);
            vertices.push(traj.state[idx * 3 + 1] as f32);
            vertices.push(traj.state[idx * 3 + 2] as f32);
            indices.push(idx as u32);
        }

        out.push(GeometryBuffer {
            vertex_buffer: vertices,
            normal_buffer: Vec::new(),
            index_buffer: indices,
            uv_buffer: Vec::new(),
            layer_id: traj.layer_id,
            is_delta: false,
        });
    }

    Ok(out)
}
