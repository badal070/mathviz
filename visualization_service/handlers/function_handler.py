from __future__ import annotations

from visualization_service.handlers._common import density_to_steps, layer_meta_list, numeric_parameters, stable_hash
from visualization_service.handlers.base import ComputationSpec, ConceptHandler, LayerMetadata
from visualization_service.schema.enums import ConceptType
from visualization_service.schema.step_descriptor import StepDescriptor


class FunctionHandler(ConceptHandler):
    def build_computation_spec(self, step: StepDescriptor, domain_arrays: dict, parsed_asts: list[dict]) -> ComputationSpec:
        if not parsed_asts:
            raise ValueError("FunctionHandler requires at least one parsed AST")
        if step.domain is None:
            raise ValueError("Function step requires domain")

        domain = step.domain.clamped().model_dump(mode="json")
        params = numeric_parameters(step)

        concept = step.concept_type
        if concept in {ConceptType.FUNCTION_2D, ConceptType.EQUATION_BUILDUP, ConceptType.SERIES_CONVERGENCE}:
            entries = [
                {
                    "hash_key": stable_hash({"s": step.step_index, "i": i, "ast": ast, "d": domain}),
                    "ast": ast,
                    "domain": domain,
                    "concept_type": "curve_2d",
                    "layer_id": f"function2d_{step.step_index}_{i}",
                }
                for i, ast in enumerate(parsed_asts)
            ]
            return ComputationSpec(
                rust_function_name="batch_evaluate",
                request_payload={
                    "entries": entries,
                    "parameters": params,
                    "allow_non_finite": False,
                },
            )

        # FUNCTION_3D and fallback surface-like concepts.
        density_steps = density_to_steps(step.render_hints.surface_density)
        if domain.get("x") is not None:
            domain["x"]["steps"] = min(max(64, density_steps), domain["x"]["steps"])
        if domain.get("y") is not None:
            domain["y"]["steps"] = min(max(64, density_steps), domain["y"]["steps"])

        entries = [
            {
                "hash_key": stable_hash({"s": step.step_index, "i": i, "ast": ast, "d": domain}),
                "ast": ast,
                "domain": domain,
                "concept_type": "explicit_surface",
                "layer_id": f"function3d_{step.step_index}_{i}",
            }
            for i, ast in enumerate(parsed_asts)
        ]
        return ComputationSpec(
            rust_function_name="batch_evaluate",
            request_payload={
                "entries": entries,
                "parameters": params,
                "allow_non_finite": False,
            },
        )

    def build_layer_metadata(self, step: StepDescriptor) -> list[LayerMetadata]:
        prefix = "function3d" if step.concept_type == ConceptType.FUNCTION_3D else "function2d"
        if isinstance(step.expression, list):
            layer_ids = [f"{prefix}_{step.step_index}_{i}" for i in range(len(step.expression))]
        else:
            layer_ids = [f"{prefix}_{step.step_index}_0"]
        return layer_meta_list(step, layer_ids)
