use thiserror::Error;

#[derive(Debug, Error, Clone)]
pub enum MathvizError {
    #[error("domain violation: {0}")]
    DomainViolation(String),
    #[error("evaluation error: {0}")]
    EvalError(String),
    #[error("mesh error: {0}")]
    MeshError(String),
    #[error("ODE error: {0}")]
    OdeError(String),
    #[error("deserialize error: {0}")]
    DeserializeError(String),
    #[error("unsupported operation: {0}")]
    UnsupportedOperation(String),
}

pub type MathvizResult<T> = Result<T, MathvizError>;
