from __future__ import annotations

from visualization_service.handlers._common import layer_meta_list, numeric_parameters
from visualization_service.handlers.base import ComputationSpec, ConceptHandler, LayerMetadata
from visualization_service.schema.enums import ConceptType
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

        include_differentials = step.render_hints.show_normals or step.concept_type == ConceptType.VECTOR_FIELD_3D
        include_streamlines = True
        params = numeric_parameters(step)

        payload = {
            "p_ast": p_ast,
            "q_ast": q_ast,
            "r_ast": r_ast,
            "domain": step.domain.clamped().model_dump(mode="json"),
            "parameters": params,
            "include_differentials": include_differentials,
            "include_streamlines": include_streamlines,
            "streamline_seeds": [],
            "streamline_max_steps": int(params.get("streamline_max_steps", 400)),
            "streamline_step": float(params.get("streamline_step", 0.02)),
            "layer_id": f"vector_{step.step_index}",
            "allow_non_finite": False,
        }
        return ComputationSpec(rust_function_name="process_vector_field", request_payload=payload)

    def build_layer_metadata(self, step: StepDescriptor) -> list[LayerMetadata]:
        # Unit arrow + potential streamline layers (dynamic count at runtime).
        return layer_meta_list(step, [f"vector_{step.step_index}_unit"])
