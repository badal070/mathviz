from __future__ import annotations

import json

from visualization_service.handlers._common import default_layer_meta
from visualization_service.handlers.base import ComputationSpec, ConceptHandler, LayerMetadata
from visualization_service.schema.step_descriptor import StepDescriptor


class LinalgHandler(ConceptHandler):
    def build_computation_spec(self, step: StepDescriptor, domain_arrays: dict, parsed_asts: list[dict]) -> ComputationSpec:
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

    def build_layer_metadata(self, step: StepDescriptor) -> LayerMetadata:
        return default_layer_meta(step, f"linalg_{step.step_index}")
