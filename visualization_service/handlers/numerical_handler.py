from __future__ import annotations

from visualization_service.handlers._common import layer_meta_list, numeric_parameters
from visualization_service.handlers.base import ComputationSpec, ConceptHandler, LayerMetadata
from visualization_service.schema.enums import ConceptType
from visualization_service.schema.step_descriptor import StepDescriptor


class NumericalHandler(ConceptHandler):
    def build_computation_spec(self, step: StepDescriptor, domain_arrays: dict, parsed_asts: list[dict]) -> ComputationSpec:
        if step.concept_type == ConceptType.RIEMANN_SUM:
            if not parsed_asts or step.domain is None:
                raise ValueError("riemann_sum requires expression and domain")
            params = numeric_parameters(step)
            payload = {
                "ast": parsed_asts[0],
                "domain": step.domain.clamped().model_dump(mode="json"),
                "subdivisions": int(params.get("n", 32)),
                "method": str(step.parameters.get("method", "midpoint")),
                "from_index": int(params.get("from_index", 0)),
                "parameters": params,
                "layer_id": f"riemann_{step.step_index}",
                "allow_non_finite": False,
            }
            return ComputationSpec(rust_function_name="generate_riemann", request_payload=payload)

        if not parsed_asts or step.domain is None:
            raise ValueError("limit_approach requires expression and domain")
        payload = {
            "ast": parsed_asts[0],
            "domain": step.domain.clamped().model_dump(mode="json"),
            "parameters": numeric_parameters(step),
            "discontinuity_threshold_factor": 10.0,
        }
        return ComputationSpec(rust_function_name="trace_curve", request_payload=payload)

    def build_layer_metadata(self, step: StepDescriptor) -> list[LayerMetadata]:
        prefix = "riemann" if step.concept_type == ConceptType.RIEMANN_SUM else "limit"
        return layer_meta_list(step, [f"{prefix}_{step.step_index}"])
