#![allow(clippy::useless_conversion)]

pub mod curve;
mod error;
pub mod evaluator;
pub mod linalg;
mod mesh;
pub mod ode;
pub mod riemann;
pub mod types;
pub mod vector_field;

use std::sync::OnceLock;

use numpy::IntoPyArray;
use pyo3::exceptions::{PyRuntimeError, PyValueError};
use pyo3::prelude::*;
use pyo3::types::PyDict;

use crate::curve::tracer::trace_explicit_curve;
use crate::error::MathvizError;
use crate::evaluator::evaluate_batch;
use crate::types::{
    BatchRequest, CurveTraceRequest, GeometryBuffer, LinearTransformRequest, LinearTransformResponse,
    OdeBatchRequest, RiemannRequest, TrajectoryBuffer, VectorFieldRequest, VectorFieldResponse,
};

static RAYON_INIT: OnceLock<usize> = OnceLock::new();

impl From<MathvizError> for PyErr {
    fn from(value: MathvizError) -> Self {
        match value {
            MathvizError::DomainViolation(msg) => PyValueError::new_err(msg),
            MathvizError::DeserializeError(msg) => PyValueError::new_err(msg),
            MathvizError::EvalError(msg) => PyRuntimeError::new_err(msg),
            MathvizError::MeshError(msg) => PyRuntimeError::new_err(msg),
            MathvizError::OdeError(msg) => PyRuntimeError::new_err(msg),
            MathvizError::UnsupportedOperation(msg) => PyRuntimeError::new_err(msg),
        }
    }
}

#[pyfunction]
#[pyo3(text_signature = "(num_threads)")]
fn configure(num_threads: usize) -> PyResult<bool> {
    if num_threads == 0 {
        return Err(PyValueError::new_err("num_threads must be > 0"));
    }

    if RAYON_INIT.get().is_some() {
        return Ok(false);
    }

    rayon::ThreadPoolBuilder::new()
        .num_threads(num_threads)
        .build_global()
        .map_err(|err| PyRuntimeError::new_err(format!("rayon init failed: {err}")))?;

    let _ = RAYON_INIT.set(num_threads);
    Ok(true)
}

#[pyfunction]
#[pyo3(text_signature = "(request_json)")]
fn batch_evaluate(py: Python<'_>, request_json: &str) -> PyResult<PyObject> {
    let request: BatchRequest =
        serde_json::from_str(request_json).map_err(|e| MathvizError::DeserializeError(e.to_string()))?;

    let result = py.allow_threads(|| evaluate_batch(request));
    let py_result = PyDict::new_bound(py);

    for (hash, outcome) in result {
        let entry = PyDict::new_bound(py);
        match (outcome.ok, outcome.err) {
            (Some(geometry), None) => {
                entry.set_item("ok", geometry_to_pydict(py, &geometry)?)?;
                entry.set_item("err", py.None())?;
            }
            (None, Some(err)) => {
                entry.set_item("ok", py.None())?;
                entry.set_item("err", err)?;
            }
            _ => {
                entry.set_item("ok", py.None())?;
                entry.set_item("err", "invalid internal outcome")?;
            }
        }
        py_result.set_item(hash, entry)?;
    }

    Ok(py_result.into_py(py))
}

#[pyfunction]
#[pyo3(text_signature = "(request_json)")]
fn trace_curve(py: Python<'_>, request_json: &str) -> PyResult<PyObject> {
    let request: CurveTraceRequest =
        serde_json::from_str(request_json).map_err(|e| MathvizError::DeserializeError(e.to_string()))?;

    let response = py.allow_threads(|| trace_explicit_curve(request))?;
    let out = PyDict::new_bound(py);
    out.set_item("geometry", geometry_to_pydict(py, &response.geometry)?)?;
    out.set_item("arc_length", response.arc_length.into_pyarray_bound(py))?;
    Ok(out.into_py(py))
}

#[pyfunction]
#[pyo3(text_signature = "(ivps_json)")]
fn solve_ode_batch(py: Python<'_>, ivps_json: &str) -> PyResult<PyObject> {
    let request: OdeBatchRequest =
        serde_json::from_str(ivps_json).map_err(|e| MathvizError::DeserializeError(e.to_string()))?;
    let trajectories = py.allow_threads(|| ode::solve_batch(request))?;

    let out = pyo3::types::PyList::empty_bound(py);
    for traj in trajectories {
        out.append(trajectory_to_pydict(py, &traj)?)?;
    }
    Ok(out.into_py(py))
}

#[pyfunction]
#[pyo3(text_signature = "(request_json)")]
fn process_vector_field(py: Python<'_>, request_json: &str) -> PyResult<PyObject> {
    let request: VectorFieldRequest =
        serde_json::from_str(request_json).map_err(|e| MathvizError::DeserializeError(e.to_string()))?;
    let response = py.allow_threads(|| vector_field::process(request))?;
    Ok(vector_field_to_pydict(py, &response)?.into_py(py))
}

#[pyfunction]
#[pyo3(text_signature = "(request_json)")]
fn generate_riemann(py: Python<'_>, request_json: &str) -> PyResult<PyObject> {
    let request: RiemannRequest =
        serde_json::from_str(request_json).map_err(|e| MathvizError::DeserializeError(e.to_string()))?;
    let geometry = py.allow_threads(|| riemann::generate(request))?;
    Ok(geometry_to_pydict(py, &geometry)?.into_py(py))
}

#[pyfunction]
#[pyo3(text_signature = "(matrix_json, domain_json)")]
fn visualize_linear_transform(py: Python<'_>, matrix_json: &str, domain_json: &str) -> PyResult<PyObject> {
    let matrix: Vec<Vec<f64>> =
        serde_json::from_str(matrix_json).map_err(|e| MathvizError::DeserializeError(e.to_string()))?;
    let domain: crate::types::DomainSpec =
        serde_json::from_str(domain_json).map_err(|e| MathvizError::DeserializeError(e.to_string()))?;

    let request = LinearTransformRequest {
        matrix,
        domain,
        grid_density: None,
        layer_id: "linear_transform".to_string(),
    };
    let response = py.allow_threads(|| linalg::visualize(request))?;
    Ok(linear_transform_to_pydict(py, &response)?.into_py(py))
}

fn geometry_to_pydict<'py>(py: Python<'py>, geometry: &GeometryBuffer) -> PyResult<Bound<'py, PyDict>> {
    let dict = PyDict::new_bound(py);
    dict.set_item(
        "vertex_buffer",
        geometry.vertex_buffer.clone().into_pyarray_bound(py),
    )?;
    dict.set_item(
        "normal_buffer",
        geometry.normal_buffer.clone().into_pyarray_bound(py),
    )?;
    dict.set_item(
        "index_buffer",
        geometry.index_buffer.clone().into_pyarray_bound(py),
    )?;
    dict.set_item("uv_buffer", geometry.uv_buffer.clone().into_pyarray_bound(py))?;
    dict.set_item("layer_id", geometry.layer_id.clone())?;
    dict.set_item("is_delta", geometry.is_delta)?;
    Ok(dict)
}

fn trajectory_to_pydict<'py>(py: Python<'py>, trajectory: &TrajectoryBuffer) -> PyResult<Bound<'py, PyDict>> {
    let dict = PyDict::new_bound(py);
    dict.set_item("state", trajectory.state.clone().into_pyarray_bound(py))?;
    dict.set_item("times", trajectory.times.clone().into_pyarray_bound(py))?;
    dict.set_item("dimension", trajectory.dimension)?;
    dict.set_item("layer_id", trajectory.layer_id.clone())?;
    dict.set_item("terminated_reason", trajectory.terminated_reason.clone())?;
    Ok(dict)
}

fn vector_field_to_pydict<'py>(py: Python<'py>, response: &VectorFieldResponse) -> PyResult<Bound<'py, PyDict>> {
    let dict = PyDict::new_bound(py);
    dict.set_item("unit_arrow", geometry_to_pydict(py, &response.unit_arrow)?)?;
    dict.set_item(
        "instance_buffer",
        response.instance_buffer.data.clone().into_pyarray_bound(py),
    )?;
    dict.set_item("divergence", response.divergence.clone().into_pyarray_bound(py))?;
    dict.set_item("curl", response.curl.clone().into_pyarray_bound(py))?;

    let streamlines = pyo3::types::PyList::empty_bound(py);
    for line in &response.streamlines {
        streamlines.append(geometry_to_pydict(py, line)?)?;
    }
    dict.set_item("streamlines", streamlines)?;
    Ok(dict)
}

fn linear_transform_to_pydict<'py>(
    py: Python<'py>,
    response: &LinearTransformResponse,
) -> PyResult<Bound<'py, PyDict>> {
    let dict = PyDict::new_bound(py);
    dict.set_item("before", geometry_to_pydict(py, &response.before)?)?;
    dict.set_item("after", geometry_to_pydict(py, &response.after)?)?;

    let eigen_layers = pyo3::types::PyList::empty_bound(py);
    for layer in &response.eigen_layers {
        eigen_layers.append(geometry_to_pydict(py, layer)?)?;
    }
    dict.set_item("eigen_layers", eigen_layers)?;

    let svd_layers = pyo3::types::PyList::empty_bound(py);
    for layer in &response.svd_layers {
        svd_layers.append(geometry_to_pydict(py, layer)?)?;
    }
    dict.set_item("svd_layers", svd_layers)?;
    Ok(dict)
}

#[pymodule]
fn mathviz_core(_py: Python<'_>, m: &Bound<'_, PyModule>) -> PyResult<()> {
    m.add_function(wrap_pyfunction!(configure, m)?)?;
    m.add_function(wrap_pyfunction!(batch_evaluate, m)?)?;
    m.add_function(wrap_pyfunction!(trace_curve, m)?)?;
    m.add_function(wrap_pyfunction!(solve_ode_batch, m)?)?;
    m.add_function(wrap_pyfunction!(process_vector_field, m)?)?;
    m.add_function(wrap_pyfunction!(generate_riemann, m)?)?;
    m.add_function(wrap_pyfunction!(visualize_linear_transform, m)?)?;
    Ok(())
}

#[cfg(test)]
mod tests {
    use crate::{
        evaluator::ast::eval_ast,
        types::{ASTNode, AxisSpec, BinaryOp, DomainSpec},
    };

    #[test]
    fn eval_simple_ast() {
        let ast = ASTNode::Binary {
            op: BinaryOp::Add,
            left: Box::new(ASTNode::Variable {
                name: "x".to_string(),
            }),
            right: Box::new(ASTNode::Literal { value: 2.0 }),
        };

        let out = eval_ast(&ast, &[("x", 3.0)], false).expect("eval should succeed");
        assert!((out - 5.0).abs() < 1e-12);
    }

    #[test]
    fn domain_clamps_steps() {
        let domain = DomainSpec {
            x: Some(AxisSpec {
                min: -1.0,
                max: 1.0,
                steps: 8,
            }),
            y: None,
            z: None,
            t: None,
        };

        let validated = domain.validate_and_clamp().expect("domain should validate");
        assert_eq!(validated.x.expect("x axis").steps, 64);
    }
}
