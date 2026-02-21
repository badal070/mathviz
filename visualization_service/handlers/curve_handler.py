from __future__ import annotations

from visualization_service.handlers._common import default_layer_meta
from visualization_service.handlers.base import ComputationSpec, ConceptHandler, LayerMetadata
from visualization_service.schema.step_descriptor import StepDescriptor


class CurveHandler(ConceptHandler):
    def build_computation_spec(self, step: StepDescriptor, domain_arrays: dict, parsed_asts: list[dict]) -> ComputationSpec:
        if not parsed_asts:
            raise ValueError("CurveHandler requires expression")
        if step.domain is None:
            raise ValueError("Curve step requires domain")

        payload = {
            "ast": parsed_asts[0],
            "domain": step.domain.clamped().model_dump(mode="json"),
            "parameters": {
                k: float(v)
                for k, v in step.parameters.items()
                if isinstance(v, (int, float))
            },
            "discontinuity_threshold_factor": 10.0,
        }
        return ComputationSpec(rust_function_name="trace_curve", request_payload=payload)

    def build_layer_metadata(self, step: StepDescriptor) -> LayerMetadata:
        return default_layer_meta(step, f"curve_{step.step_index}")
