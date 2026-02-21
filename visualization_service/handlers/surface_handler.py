from __future__ import annotations

from visualization_service.handlers._common import default_layer_meta
from visualization_service.handlers.base import ComputationSpec, ConceptHandler, LayerMetadata
from visualization_service.schema.enums import ConceptType
from visualization_service.schema.step_descriptor import StepDescriptor


class SurfaceHandler(ConceptHandler):
    def build_computation_spec(self, step: StepDescriptor, domain_arrays: dict, parsed_asts: list[dict]) -> ComputationSpec:
        if not parsed_asts:
            raise ValueError("SurfaceHandler requires expression")
        if step.domain is None:
            raise ValueError("Surface step requires domain")

        concept_hint = "explicit_surface"
        if step.concept_type == ConceptType.IMPLICIT_SURFACE:
            concept_hint = "implicit_surface"

        payload = {
            "entries": [
                {
                    "hash_key": f"surface_{step.step_index}",
                    "ast": parsed_asts[0],
                    "domain": step.domain.clamped().model_dump(mode="json"),
                    "concept_type": concept_hint,
                    "layer_id": f"surface_{step.step_index}",
                }
            ],
            "parameters": {
                k: float(v)
                for k, v in step.parameters.items()
                if isinstance(v, (int, float))
            },
            "allow_non_finite": step.concept_type == ConceptType.IMPLICIT_SURFACE,
        }
        return ComputationSpec(rust_function_name="batch_evaluate", request_payload=payload)

    def build_layer_metadata(self, step: StepDescriptor) -> LayerMetadata:
        return default_layer_meta(step, f"surface_{step.step_index}")
