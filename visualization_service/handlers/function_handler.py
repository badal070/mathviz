from __future__ import annotations

from visualization_service.handlers._common import default_layer_meta
from visualization_service.handlers.base import ComputationSpec, ConceptHandler, LayerMetadata
from visualization_service.schema.step_descriptor import StepDescriptor


class FunctionHandler(ConceptHandler):
    def build_computation_spec(self, step: StepDescriptor, domain_arrays: dict, parsed_asts: list[dict]) -> ComputationSpec:
        if not parsed_asts:
            raise ValueError("FunctionHandler requires at least one parsed AST")
        domain = (step.domain.clamped() if step.domain is not None else None)
        if domain is None:
            raise ValueError("Step requires domain")

        payload = {
            "entries": [
                {
                    "hash_key": f"{step.step_index}_{i}",
                    "ast": ast,
                    "domain": domain.model_dump(mode="json"),
                    "concept_type": "explicit_surface" if step.concept_type.value.endswith("3d") else "curve_2d",
                    "layer_id": f"layer_{step.step_index}_{i}",
                }
                for i, ast in enumerate(parsed_asts)
            ],
            "parameters": {
                k: float(v)
                for k, v in step.parameters.items()
                if isinstance(v, (int, float))
            },
            "allow_non_finite": False,
        }
        return ComputationSpec(rust_function_name="batch_evaluate", request_payload=payload)

    def build_layer_metadata(self, step: StepDescriptor) -> LayerMetadata:
        return default_layer_meta(step, f"layer_{step.step_index}_0")
