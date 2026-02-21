use mathviz_core::types::{
    ASTNode, AxisSpec, BatchEntry, BatchRequest, ConceptTypeHint, CurveTraceRequest, DomainSpec, IVPSpec,
    LinearTransformRequest, OdeBatchRequest, OdeMethod, PartitionMethod, Point3, RiemannRequest,
    VectorFieldRequest,
};

#[test]
fn explicit_surface_vertex_and_index_counts() {
    let ast = ASTNode::Binary {
        op: mathviz_core::types::BinaryOp::Add,
        left: Box::new(ASTNode::Binary {
            op: mathviz_core::types::BinaryOp::Pow,
            left: Box::new(ASTNode::Variable {
                name: "x".to_string(),
            }),
            right: Box::new(ASTNode::Literal { value: 2.0 }),
        }),
        right: Box::new(ASTNode::Binary {
            op: mathviz_core::types::BinaryOp::Pow,
            left: Box::new(ASTNode::Variable {
                name: "y".to_string(),
            }),
            right: Box::new(ASTNode::Literal { value: 2.0 }),
        }),
    };

    let domain = DomainSpec {
        x: Some(AxisSpec {
            min: -2.0,
            max: 2.0,
            steps: 64,
        }),
        y: Some(AxisSpec {
            min: -2.0,
            max: 2.0,
            steps: 64,
        }),
        z: None,
        t: None,
    };

    let req = BatchRequest {
        entries: vec![BatchEntry {
            hash_key: "paraboloid".to_string(),
            ast,
            domain,
            concept_type: ConceptTypeHint::ExplicitSurface,
            layer_id: "paraboloid".to_string(),
        }],
        ..Default::default()
    };

    let out = mathviz_core::evaluator::evaluate_batch(req);
    let geom = out
        .get("paraboloid")
        .and_then(|o| o.ok.as_ref())
        .expect("geometry expected");

    assert_eq!(geom.vertex_buffer.len(), 64 * 64 * 3);
    assert_eq!(geom.normal_buffer.len(), 64 * 64 * 3);
    assert!(!geom.index_buffer.is_empty());
}

#[test]
fn curve_arc_length_is_monotonic_and_normalized() {
    let request = CurveTraceRequest {
        ast: ASTNode::Unary {
            op: mathviz_core::types::UnaryOp::Sin,
            child: Box::new(ASTNode::Variable {
                name: "x".to_string(),
            }),
        },
        domain: DomainSpec {
            x: Some(AxisSpec {
                min: 0.0,
                max: std::f64::consts::TAU,
                steps: 256,
            }),
            y: None,
            z: None,
            t: None,
        },
        parameters: Default::default(),
        discontinuity_threshold_factor: 10.0,
    };

    let out = mathviz_core::curve::tracer::trace_explicit_curve(request).expect("curve trace");
    let mut prev = -1.0f32;
    for &v in &out.arc_length {
        assert!(v >= prev);
        prev = v;
    }
    assert!((out.arc_length.last().copied().unwrap_or_default() - 1.0).abs() < 1e-5);
}

#[test]
fn ode_rk4_matches_exp_growth() {
    // y' = y, y(0)=1 => y(t)=e^t
    let ivp = IVPSpec {
        derivatives: vec![ASTNode::Variable {
            name: "x".to_string(),
        }],
        initial_state: vec![1.0],
        t0: 0.0,
        t_end: 1.0,
        method: Some(OdeMethod::Rk4),
        step_size: Some(0.01),
        max_steps: Some(20_000),
        abs_tol: None,
        rel_tol: None,
        h_min: None,
        h_max: None,
        domain: None,
        layer_id: "rk4_exp".to_string(),
    };

    let out = mathviz_core::ode::solve_batch(OdeBatchRequest {
        ivps: vec![ivp],
        ..Default::default()
    })
    .expect("rk4 solve should succeed");

    let traj = &out[0];
    let final_x = traj.state[traj.state.len() - 3];
    let expected = std::f64::consts::E;
    assert!((final_x - expected).abs() < 2e-4);
}

#[test]
fn ode_rk45_matches_exp_growth() {
    // y' = y, y(0)=1 => y(t)=e^t
    let ivp = IVPSpec {
        derivatives: vec![ASTNode::Variable {
            name: "x".to_string(),
        }],
        initial_state: vec![1.0],
        t0: 0.0,
        t_end: 1.0,
        method: Some(OdeMethod::Rk45),
        step_size: Some(0.05),
        max_steps: Some(20_000),
        abs_tol: Some(1e-9),
        rel_tol: Some(1e-8),
        h_min: Some(1e-8),
        h_max: Some(0.2),
        domain: None,
        layer_id: "rk45_exp".to_string(),
    };

    let out = mathviz_core::ode::solve_batch(OdeBatchRequest {
        ivps: vec![ivp],
        ..Default::default()
    })
    .expect("rk45 solve should succeed");

    let traj = &out[0];
    let final_x = traj.state[traj.state.len() - 3];
    let expected = std::f64::consts::E;
    assert!((final_x - expected).abs() < 1e-6);
}

#[test]
fn riemann_generator_emits_incremental_geometry() {
    let req = RiemannRequest {
        ast: ASTNode::Variable {
            name: "x".to_string(),
        },
        domain: DomainSpec {
            x: Some(AxisSpec {
                min: 0.0,
                max: 1.0,
                steps: 128,
            }),
            y: None,
            z: None,
            t: None,
        },
        subdivisions: 10,
        method: Some(PartitionMethod::Midpoint),
        from_index: Some(5),
        parameters: Default::default(),
        layer_id: "riemann_test".to_string(),
        allow_non_finite: false,
    };

    let out = mathviz_core::riemann::generate(req).expect("riemann generation");
    assert_eq!(out.vertex_buffer.len(), 5 * 4 * 3);
    assert_eq!(out.index_buffer.len(), 5 * 6);
    assert!(out.is_delta);
}

#[test]
fn vector_field_processor_generates_instances() {
    let request = VectorFieldRequest {
        p_ast: ASTNode::Variable {
            name: "x".to_string(),
        },
        q_ast: ASTNode::Variable {
            name: "y".to_string(),
        },
        r_ast: ASTNode::Variable {
            name: "z".to_string(),
        },
        domain: DomainSpec {
            x: Some(AxisSpec {
                min: -1.0,
                max: 1.0,
                steps: 64,
            }),
            y: Some(AxisSpec {
                min: -1.0,
                max: 1.0,
                steps: 64,
            }),
            z: Some(AxisSpec {
                min: -1.0,
                max: 1.0,
                steps: 64,
            }),
            t: None,
        },
        parameters: Default::default(),
        include_differentials: true,
        include_streamlines: true,
        streamline_seeds: vec![Point3 {
            x: 0.1,
            y: 0.2,
            z: 0.3,
        }],
        streamline_max_steps: Some(100),
        streamline_step: Some(0.02),
        layer_id: "vf".to_string(),
        allow_non_finite: false,
    };

    let out = mathviz_core::vector_field::process(request).expect("vector field process");
    assert!(!out.instance_buffer.data.is_empty());
    assert!(!out.divergence.is_empty());
    assert!(!out.curl.is_empty());
    assert_eq!(out.streamlines.len(), 1);
}

#[test]
fn linear_transform_visualizer_returns_layers() {
    let request = LinearTransformRequest {
        matrix: vec![vec![2.0, 0.0], vec![0.0, 0.5]],
        domain: DomainSpec {
            x: Some(AxisSpec {
                min: -1.0,
                max: 1.0,
                steps: 64,
            }),
            y: Some(AxisSpec {
                min: -1.0,
                max: 1.0,
                steps: 64,
            }),
            z: None,
            t: None,
        },
        grid_density: Some(8),
        layer_id: "lin".to_string(),
    };

    let out = mathviz_core::linalg::visualize(request).expect("linalg visualize");
    assert!(!out.before.vertex_buffer.is_empty());
    assert!(!out.after.vertex_buffer.is_empty());
    assert!(!out.eigen_layers.is_empty());
    assert_eq!(out.svd_layers.len(), 3);
}
