use mathviz_core::types::{
    ASTNode, AxisSpec, BatchEntry, BatchRequest, ConceptTypeHint, CurveTraceRequest, DomainSpec,
    IVPSpec, LinearTransformRequest, OdeBatchRequest, OdeMethod, PartitionMethod, Point3,
    RiemannRequest, VectorFieldRequest,
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
fn implicit_surface_generates_triangles_and_normals() {
    let ast = ASTNode::Binary {
        op: mathviz_core::types::BinaryOp::Sub,
        left: Box::new(ASTNode::Nary {
            op: mathviz_core::types::NaryOp::Sum,
            children: vec![
                ASTNode::Binary {
                    op: mathviz_core::types::BinaryOp::Pow,
                    left: Box::new(ASTNode::Variable {
                        name: "x".to_string(),
                    }),
                    right: Box::new(ASTNode::Literal { value: 2.0 }),
                },
                ASTNode::Binary {
                    op: mathviz_core::types::BinaryOp::Pow,
                    left: Box::new(ASTNode::Variable {
                        name: "y".to_string(),
                    }),
                    right: Box::new(ASTNode::Literal { value: 2.0 }),
                },
                ASTNode::Binary {
                    op: mathviz_core::types::BinaryOp::Pow,
                    left: Box::new(ASTNode::Variable {
                        name: "z".to_string(),
                    }),
                    right: Box::new(ASTNode::Literal { value: 2.0 }),
                },
            ],
        }),
        right: Box::new(ASTNode::Literal { value: 1.0 }),
    };

    let req = BatchRequest {
        entries: vec![BatchEntry {
            hash_key: "unit_sphere".to_string(),
            ast,
            domain: DomainSpec {
                x: Some(AxisSpec {
                    min: -1.2,
                    max: 1.2,
                    steps: 64,
                }),
                y: Some(AxisSpec {
                    min: -1.2,
                    max: 1.2,
                    steps: 64,
                }),
                z: Some(AxisSpec {
                    min: -1.2,
                    max: 1.2,
                    steps: 64,
                }),
                t: None,
            },
            concept_type: ConceptTypeHint::ImplicitSurface,
            layer_id: "sphere".to_string(),
        }],
        allow_non_finite: true,
        ..Default::default()
    };

    let out = mathviz_core::evaluator::evaluate_batch(req);
    let geom = out
        .get("unit_sphere")
        .and_then(|o| o.ok.as_ref())
        .expect("implicit geometry expected");

    assert!(!geom.index_buffer.is_empty());
    assert_eq!(geom.normal_buffer.len(), geom.vertex_buffer.len());
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
        x_ast: None,
        y_ast: None,
        z_ast: None,
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
        parameter_name: None,
        allow_non_finite: false,
        layer_id: "curve_sin".to_string(),
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
fn parametric_curve_supports_t_axis_and_cusp_tagging() {
    let t_var = ASTNode::Variable {
        name: "t".to_string(),
    };
    let request = CurveTraceRequest {
        ast: ASTNode::Literal { value: 0.0 },
        x_ast: Some(ASTNode::Binary {
            op: mathviz_core::types::BinaryOp::Pow,
            left: Box::new(t_var.clone()),
            right: Box::new(ASTNode::Literal { value: 2.0 }),
        }),
        y_ast: Some(ASTNode::Binary {
            op: mathviz_core::types::BinaryOp::Pow,
            left: Box::new(t_var),
            right: Box::new(ASTNode::Literal { value: 3.0 }),
        }),
        z_ast: None,
        domain: DomainSpec {
            x: None,
            y: None,
            z: None,
            t: Some(AxisSpec {
                min: -1.0,
                max: 1.0,
                steps: 257,
            }),
        },
        parameters: Default::default(),
        parameter_name: Some("t".to_string()),
        allow_non_finite: false,
        layer_id: "cusp_curve".to_string(),
        discontinuity_threshold_factor: 10.0,
    };

    let out =
        mathviz_core::curve::tracer::trace_explicit_curve(request).expect("parametric curve trace");
    assert_eq!(out.geometry.vertex_buffer.len(), 257 * 3);
    assert!(out.geometry.index_buffer.contains(&u32::MAX) || !out.geometry.index_buffer.is_empty());
    assert!(!out.cusp_indices.is_empty());
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

#[test]
fn linear_transform_3d_generates_full_lattice_and_svd_line_layers() {
    let density = 4usize;
    let request = LinearTransformRequest {
        matrix: vec![
            vec![1.0, 0.2, 0.0],
            vec![0.0, 1.0, 0.1],
            vec![0.0, 0.0, 1.0],
        ],
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
        grid_density: Some(density),
        layer_id: "lin3".to_string(),
    };

    let out = mathviz_core::linalg::visualize(request).expect("3d linalg visualize");
    let line_count = 3 * density * density;
    assert_eq!(out.before.vertex_buffer.len(), line_count * 2 * 3);
    assert_eq!(out.before.index_buffer.len(), line_count * 3);
    assert!(out.before.index_buffer.contains(&u32::MAX));

    assert_eq!(out.svd_layers.len(), 3);
    for layer in &out.svd_layers {
        assert!(layer.index_buffer.contains(&u32::MAX));
    }
}

#[test]
fn linear_transform_complex_2x2_emits_rotation_scale_indicator() {
    let request = LinearTransformRequest {
        // 90-degree rotation has complex eigenvalues.
        matrix: vec![vec![0.0, -1.0], vec![1.0, 0.0]],
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
        layer_id: "complex_lin".to_string(),
    };

    let out = mathviz_core::linalg::visualize(request).expect("complex 2x2 visualize");
    assert_eq!(out.eigen_layers.len(), 1);
    assert!(!out.eigen_layers[0].vertex_buffer.is_empty());
    assert!(out.eigen_layers[0].index_buffer.contains(&u32::MAX));
}
