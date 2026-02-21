from __future__ import annotations

from visualization_service.handlers._common import density_to_steps, layer_meta_list, numeric_parameters, stable_hash
from visualization_service.handlers.base import ComputationSpec, ConceptHandler, LayerMetadata
from visualization_service.schema.enums import ConceptType
from visualization_service.schema.step_descriptor import StepDescriptor


class SurfaceHandler(ConceptHandler):
    def build_computation_spec(self, step: StepDescriptor, domain_arrays: dict, parsed_asts: list[dict]) -> ComputationSpec:
        if not parsed_asts:
            raise ValueError("SurfaceHandler requires expression")
        if step.domain is None:
            raise ValueError("Surface step requires domain")

        domain = step.domain.clamped().model_dump(mode="json")
        density_steps = density_to_steps(step.render_hints.surface_density)
        if domain.get("x") is not None:
            domain["x"]["steps"] = min(max(64, density_steps), domain["x"]["steps"])
        if domain.get("y") is not None:
            domain["y"]["steps"] = min(max(64, density_steps), domain["y"]["steps"])
        if domain.get("z") is not None:
            domain["z"]["steps"] = min(max(64, density_steps // 2), domain["z"]["steps"])

        concept_hint = "explicit_surface"
        allow_non_finite = False
        if step.concept_type == ConceptType.IMPLICIT_SURFACE:
            concept_hint = "implicit_surface"
            allow_non_finite = True

        entries = [
            {
                "hash_key": stable_hash({"s": step.step_index, "i": i, "ast": ast, "d": domain, "t": concept_hint}),
                "ast": ast,
                "domain": domain,
                "concept_type": concept_hint,
                "layer_id": f"surface_{step.step_index}_{i}",
            }
            for i, ast in enumerate(parsed_asts)
        ]

        return ComputationSpec(
            rust_function_name="batch_evaluate",
            request_payload={
                "entries": entries,
                "parameters": numeric_parameters(step),
                "allow_non_finite": allow_non_finite,
            },
        )

    def build_layer_metadata(self, step: StepDescriptor) -> list[LayerMetadata]:
        count = len(step.expression) if isinstance(step.expression, list) else 1
        return layer_meta_list(step, [f"surface_{step.step_index}_{i}" for i in range(count)])
