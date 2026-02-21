from __future__ import annotations

from visualization_service.handlers._common import layer_meta_list, numeric_parameters, stable_hash
from visualization_service.handlers.base import ComputationSpec, ConceptHandler, LayerMetadata
from visualization_service.schema.step_descriptor import StepDescriptor


class TopologyHandler(ConceptHandler):
    def build_computation_spec(self, step: StepDescriptor, domain_arrays: dict, parsed_asts: list[dict]) -> ComputationSpec:
        if not parsed_asts:
            raise ValueError("manifold requires expression")
        if step.domain is None:
            raise ValueError("manifold requires domain")

        domain = step.domain.clamped().model_dump(mode="json")
        entries = [
            {
                "hash_key": stable_hash({"s": step.step_index, "ast": parsed_asts[0], "d": domain, "c": "manifold"}),
                "ast": parsed_asts[0],
                "domain": domain,
                "concept_type": "implicit_surface" if domain.get("z") else "explicit_surface",
                "layer_id": f"manifold_{step.step_index}",
            }
        ]
        return ComputationSpec(
            rust_function_name="batch_evaluate",
            request_payload={
                "entries": entries,
                "parameters": numeric_parameters(step),
                "allow_non_finite": bool(domain.get("z")),
            },
        )

    def build_layer_metadata(self, step: StepDescriptor) -> list[LayerMetadata]:
        return layer_meta_list(step, [f"manifold_{step.step_index}"])
