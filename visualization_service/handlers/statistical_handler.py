from __future__ import annotations

from visualization_service.handlers._common import layer_meta_list, numeric_parameters, stable_hash
from visualization_service.handlers.base import ComputationSpec, ConceptHandler, LayerMetadata
from visualization_service.schema.step_descriptor import StepDescriptor


class StatisticalHandler(ConceptHandler):
    def build_computation_spec(self, step: StepDescriptor, domain_arrays: dict, parsed_asts: list[dict]) -> ComputationSpec:
        if not parsed_asts:
            raise ValueError("distribution requires expression")
        if step.domain is None:
            raise ValueError("distribution requires domain")

        domain = step.domain.clamped().model_dump(mode="json")
        entries = [
            {
                "hash_key": stable_hash({"s": step.step_index, "ast": parsed_asts[0], "d": domain, "c": "distribution"}),
                "ast": parsed_asts[0],
                "domain": domain,
                "concept_type": "curve_2d",
                "layer_id": f"distribution_{step.step_index}",
            }
        ]
        return ComputationSpec(
            rust_function_name="batch_evaluate",
            request_payload={
                "entries": entries,
                "parameters": numeric_parameters(step),
                "allow_non_finite": False,
            },
        )

    def build_layer_metadata(self, step: StepDescriptor) -> list[LayerMetadata]:
        return layer_meta_list(step, [f"distribution_{step.step_index}"])
