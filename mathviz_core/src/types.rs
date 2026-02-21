use std::collections::BTreeMap;

use serde::{Deserialize, Serialize};

use crate::error::{MathvizError, MathvizResult};

pub const MIN_STEPS: usize = 64;
pub const MAX_STEPS: usize = 2048;

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "snake_case")]
pub enum UnaryOp {
    Neg,
    Sqrt,
    Abs,
    Sin,
    Cos,
    Tan,
    Asin,
    Acos,
    Atan,
    Sinh,
    Cosh,
    Tanh,
    Exp,
    Ln,
    Log10,
    Floor,
    Ceil,
    Sign,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "snake_case")]
pub enum BinaryOp {
    Add,
    Sub,
    Mul,
    Div,
    Pow,
    Atan2,
    Mod,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "snake_case")]
pub enum NaryOp {
    Sum,
    Product,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum ASTNode {
    Literal { value: f64 },
    Variable { name: String },
    Unary {
        op: UnaryOp,
        child: Box<ASTNode>,
    },
    Binary {
        op: BinaryOp,
        left: Box<ASTNode>,
        right: Box<ASTNode>,
    },
    Nary {
        op: NaryOp,
        children: Vec<ASTNode>,
    },
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct AxisSpec {
    pub min: f64,
    pub max: f64,
    pub steps: usize,
}

impl AxisSpec {
    pub fn validate_and_clamp(mut self, axis_name: &str) -> MathvizResult<Self> {
        if self.min.partial_cmp(&self.max) != Some(std::cmp::Ordering::Less) {
            return Err(MathvizError::DomainViolation(format!(
                "{axis_name}: min must be < max (got {} >= {})",
                self.min, self.max
            )));
        }
        self.steps = self.steps.clamp(MIN_STEPS, MAX_STEPS);
        Ok(self)
    }

    pub fn spacing(&self) -> f64 {
        (self.max - self.min) / ((self.steps - 1) as f64)
    }

    pub fn value_at(&self, idx: usize) -> f64 {
        self.min + (idx as f64) * self.spacing()
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Default)]
pub struct DomainSpec {
    pub x: Option<AxisSpec>,
    pub y: Option<AxisSpec>,
    pub z: Option<AxisSpec>,
    pub t: Option<AxisSpec>,
}

impl DomainSpec {
    pub fn validate_and_clamp(self) -> MathvizResult<Self> {
        let mut out = self;
        let mut axis_count = 0usize;

        if let Some(axis) = out.x.take() {
            out.x = Some(axis.validate_and_clamp("x")?);
            axis_count += 1;
        }
        if let Some(axis) = out.y.take() {
            out.y = Some(axis.validate_and_clamp("y")?);
            axis_count += 1;
        }
        if let Some(axis) = out.z.take() {
            out.z = Some(axis.validate_and_clamp("z")?);
            axis_count += 1;
        }
        if let Some(axis) = out.t.take() {
            out.t = Some(axis.validate_and_clamp("t")?);
            axis_count += 1;
        }

        if axis_count == 0 {
            return Err(MathvizError::DomainViolation(
                "at least one axis must be present".to_string(),
            ));
        }

        Ok(out)
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Default)]
pub struct GeometryBuffer {
    pub vertex_buffer: Vec<f32>,
    pub normal_buffer: Vec<f32>,
    pub index_buffer: Vec<u32>,
    pub uv_buffer: Vec<f32>,
    pub layer_id: String,
    pub is_delta: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "snake_case")]
pub enum ConceptTypeHint {
    ExplicitSurface,
    ImplicitSurface,
    Curve2d,
    Curve3d,
    Ode,
    VectorField,
    RiemannSum,
    LinearTransform,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct BatchEntry {
    pub hash_key: String,
    pub ast: ASTNode,
    pub domain: DomainSpec,
    pub concept_type: ConceptTypeHint,
    #[serde(default)]
    pub layer_id: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Default)]
pub struct BatchRequest {
    pub entries: Vec<BatchEntry>,
    #[serde(default)]
    pub parameters: BTreeMap<String, f64>,
    #[serde(default)]
    pub allow_non_finite: bool,
    #[serde(default = "default_render_bound")]
    pub render_bound: f64,
}

fn default_render_bound() -> f64 {
    1e6
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct BatchOutcome {
    pub ok: Option<GeometryBuffer>,
    pub err: Option<String>,
}

pub type BatchResult = BTreeMap<String, BatchOutcome>;

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct CurveTraceRequest {
    pub ast: ASTNode,
    pub domain: DomainSpec,
    #[serde(default)]
    pub parameters: BTreeMap<String, f64>,
    #[serde(default)]
    pub discontinuity_threshold_factor: f64,
}

impl CurveTraceRequest {
    pub fn threshold_factor(&self) -> f64 {
        if self.discontinuity_threshold_factor <= 0.0 {
            10.0
        } else {
            self.discontinuity_threshold_factor
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct CurveTraceResponse {
    pub geometry: GeometryBuffer,
    pub arc_length: Vec<f32>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "snake_case")]
pub enum OdeMethod {
    Rk4,
    Rk45,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct IVPSpec {
    pub derivatives: Vec<ASTNode>,
    pub initial_state: Vec<f64>,
    pub t0: f64,
    pub t_end: f64,
    #[serde(default)]
    pub method: Option<OdeMethod>,
    #[serde(default)]
    pub step_size: Option<f64>,
    #[serde(default)]
    pub max_steps: Option<usize>,
    #[serde(default)]
    pub abs_tol: Option<f64>,
    #[serde(default)]
    pub rel_tol: Option<f64>,
    #[serde(default)]
    pub h_min: Option<f64>,
    #[serde(default)]
    pub h_max: Option<f64>,
    #[serde(default)]
    pub domain: Option<DomainSpec>,
    #[serde(default)]
    pub layer_id: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Default)]
pub struct OdeBatchRequest {
    pub ivps: Vec<IVPSpec>,
    #[serde(default)]
    pub parameters: BTreeMap<String, f64>,
    #[serde(default)]
    pub allow_non_finite: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct TrajectoryBuffer {
    pub state: Vec<f64>,
    pub times: Vec<f64>,
    pub dimension: usize,
    pub layer_id: String,
    pub terminated_reason: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct Point3 {
    pub x: f64,
    pub y: f64,
    pub z: f64,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "snake_case")]
pub enum PartitionMethod {
    Left,
    Right,
    Midpoint,
    Trapezoid,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct RiemannRequest {
    pub ast: ASTNode,
    pub domain: DomainSpec,
    pub subdivisions: usize,
    #[serde(default)]
    pub method: Option<PartitionMethod>,
    #[serde(default)]
    pub from_index: Option<usize>,
    #[serde(default)]
    pub parameters: BTreeMap<String, f64>,
    #[serde(default)]
    pub layer_id: String,
    #[serde(default)]
    pub allow_non_finite: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct ArrowInstanceBuffer {
    pub data: Vec<f32>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct VectorFieldRequest {
    pub p_ast: ASTNode,
    pub q_ast: ASTNode,
    pub r_ast: ASTNode,
    pub domain: DomainSpec,
    #[serde(default)]
    pub parameters: BTreeMap<String, f64>,
    #[serde(default)]
    pub include_differentials: bool,
    #[serde(default)]
    pub include_streamlines: bool,
    #[serde(default)]
    pub streamline_seeds: Vec<Point3>,
    #[serde(default)]
    pub streamline_max_steps: Option<usize>,
    #[serde(default)]
    pub streamline_step: Option<f64>,
    #[serde(default)]
    pub layer_id: String,
    #[serde(default)]
    pub allow_non_finite: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct VectorFieldResponse {
    pub unit_arrow: GeometryBuffer,
    pub instance_buffer: ArrowInstanceBuffer,
    pub divergence: Vec<f32>,
    pub curl: Vec<f32>,
    pub streamlines: Vec<GeometryBuffer>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct LinearTransformRequest {
    pub matrix: Vec<Vec<f64>>,
    pub domain: DomainSpec,
    #[serde(default)]
    pub grid_density: Option<usize>,
    #[serde(default)]
    pub layer_id: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct LinearTransformResponse {
    pub before: GeometryBuffer,
    pub after: GeometryBuffer,
    pub eigen_layers: Vec<GeometryBuffer>,
    pub svd_layers: Vec<GeometryBuffer>,
}
