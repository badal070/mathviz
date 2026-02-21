from __future__ import annotations

import json

from visualization_service.handlers._common import layer_meta_list
from visualization_service.handlers.base import ComputationSpec, ConceptHandler, LayerMetadata
from visualization_service.schema.step_descriptor import StepDescriptor


class LinalgHandler(ConceptHandler):
    def build_computation_spec(self, step: StepDescriptor, domain_arrays: dict, parsed_asts: list[dict]) -> ComputationSpec:
        _ = parsed_asts
        if step.expression is None:
            raise ValueError("linear transform requires matrix expression")
        if step.domain is None:
            raise ValueError("linear transform requires domain")

        expr = step.expression if isinstance(step.expression, str) else step.expression[0]
        matrix = json.loads(expr)
        payload = {
            "matrix": matrix,
            "domain": step.domain.clamped().model_dump(mode="json"),
        }
        return ComputationSpec(rust_function_name="visualize_linear_transform", request_payload=payload)

    def build_layer_metadata(self, step: StepDescriptor) -> list[LayerMetadata]:
        return layer_meta_list(
            step,
            [
                f"linear_transform_before_{step.step_index}",
                f"linear_transform_after_{step.step_index}",
                f"linear_transform_eigen_{step.step_index}",
                f"linear_transform_svd_{step.step_index}",
            ],
        )
