from __future__ import annotations

from visualization_service.handlers._common import default_layer_meta
from visualization_service.handlers.base import ComputationSpec, ConceptHandler, LayerMetadata
from visualization_service.schema.enums import ConceptType
from visualization_service.schema.step_descriptor import StepDescriptor


class NumericalHandler(ConceptHandler):
    def build_computation_spec(self, step: StepDescriptor, domain_arrays: dict, parsed_asts: list[dict]) -> ComputationSpec:
        if step.concept_type == ConceptType.RIEMANN_SUM:
            if not parsed_asts or step.domain is None:
                raise ValueError("riemann_sum requires expression and domain")
            payload = {
                "ast": parsed_asts[0],
                "domain": step.domain.clamped().model_dump(mode="json"),
                "subdivisions": int(step.parameters.get("n", 32)) if isinstance(step.parameters.get("n", 32), (int, float)) else 32,
                "method": str(step.parameters.get("method", "midpoint")),
                "from_index": int(step.parameters.get("from_index", 0)) if isinstance(step.parameters.get("from_index", 0), (int, float)) else 0,
                "parameters": {k: float(v) for k, v in step.parameters.items() if isinstance(v, (int, float))},
                "layer_id": f"riemann_{step.step_index}",
            }
            return ComputationSpec(rust_function_name="generate_riemann", request_payload=payload)

        # limit_approach fallback through curve tracing
        if not parsed_asts or step.domain is None:
            raise ValueError("limit_approach requires expression and domain")
        payload = {
            "ast": parsed_asts[0],
            "domain": step.domain.clamped().model_dump(mode="json"),
            "parameters": {k: float(v) for k, v in step.parameters.items() if isinstance(v, (int, float))},
        }
        return ComputationSpec(rust_function_name="trace_curve", request_payload=payload)

    def build_layer_metadata(self, step: StepDescriptor) -> LayerMetadata:
        return default_layer_meta(step, f"numerical_{step.step_index}")
