# mathviz_core

Rust computation core for MathViz 3D.

## Current status

- Implemented:
  - Core data model (`ASTNode`, `DomainSpec`, `GeometryBuffer`, batch types)
  - Unified typed errors (`MathvizError`)
  - AST evaluator with depth limits and non-finite checks
  - Parallel batch evaluator with deduplication by `hash_key`
  - Explicit surface meshing (`z=f(x,y)`) and normal generation
  - Explicit curve tracing with discontinuity breaks and normalized arc-length
  - PyO3 boundary functions:
    - `configure`
    - `batch_evaluate`
    - `trace_curve`
- Scaffolded (not implemented yet):
  - Implicit surface meshing (Marching Cubes)
  - ODE solvers (RK4/RK45)
  - Vector field processing
  - Riemann sum generator
  - Linear algebra visualizations (transform/eigen/SVD)

## Build

```bash
cargo build --release
```

For maximum CPU optimization in deployment, build with `RUSTFLAGS='-C target-cpu=native'`.

## Python extension

This crate is configured for PyO3 `extension-module` and can be built with maturin in the Python service layer.
