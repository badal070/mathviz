pub mod eigen;
pub mod svd;
pub mod transform;

use crate::{
    error::{MathvizError, MathvizResult},
    types::{LinearTransformRequest, LinearTransformResponse},
};

pub fn visualize(request: LinearTransformRequest) -> MathvizResult<LinearTransformResponse> {
    let domain = request.domain.validate_and_clamp()?;
    let x_axis = domain.x.as_ref().ok_or_else(|| {
        MathvizError::DomainViolation("linear transform requires x axis".to_string())
    })?;
    let y_axis = domain.y.as_ref().ok_or_else(|| {
        MathvizError::DomainViolation("linear transform requires y axis".to_string())
    })?;
    let z_axis = domain.z.as_ref();

    let density = request.grid_density.unwrap_or(10).max(2);
    let prefix = if request.layer_id.is_empty() {
        "linear_transform"
    } else {
        &request.layer_id
    };

    let (before, after) = transform::generate_transform_layers(
        &request.matrix,
        x_axis,
        y_axis,
        z_axis,
        density,
        prefix,
    )?;
    let eigen_layers = eigen::eigen_layers(&request.matrix, prefix)?;
    let svd_layers = svd::svd_layers(&request.matrix, x_axis, y_axis, z_axis, prefix)?;

    Ok(LinearTransformResponse {
        before,
        after,
        eigen_layers,
        svd_layers,
    })
}
