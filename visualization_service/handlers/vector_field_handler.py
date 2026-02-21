from __future__ import annotations

from visualization_service.handlers._common import default_layer_meta
from visualization_service.handlers.base import ComputationSpec, ConceptHandler, LayerMetadata
from visualization_service.schema.step_descriptor import StepDescriptor


class VectorFieldHandler(ConceptHandler):
    def build_computation_spec(self, step: StepDescriptor, domain_arrays: dict, parsed_asts: list[dict]) -> ComputationSpec:
        if len(parsed_asts) not in (2, 3):
            raise ValueError("Vector field requires 2 or 3 expressions")
        if step.domain is None:
            raise ValueError("Vector field requires domain")

        p_ast = parsed_asts[0]
        q_ast = parsed_asts[1]
        r_ast = parsed_asts[2] if len(parsed_asts) == 3 else {"type": "literal", "value": 0.0}

        payload = {
            "p_ast": p_ast,
            "q_ast": q_ast,
            "r_ast": r_ast,
            "domain": step.domain.clamped().model_dump(mode="json"),
            "parameters": {
                k: float(v)
                for k, v in step.parameters.items()
                if isinstance(v, (int, float))
            },
            "include_differentials": True,
            "include_streamlines": True,
            "streamline_seeds": [],
            "layer_id": f"vector_{step.step_index}",
        }
        return ComputationSpec(rust_function_name="process_vector_field", request_payload=payload)

    def build_layer_metadata(self, step: StepDescriptor) -> LayerMetadata:
        return default_layer_meta(step, f"vector_{step.step_index}")
