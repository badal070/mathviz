from __future__ import annotations

from visualization_service.handlers._common import layer_meta_list, numeric_parameters
from visualization_service.handlers.base import ComputationSpec, ConceptHandler, LayerMetadata
from visualization_service.schema.enums import ConceptType
from visualization_service.schema.step_descriptor import StepDescriptor


class ODEHandler(ConceptHandler):
    def build_computation_spec(self, step: StepDescriptor, domain_arrays: dict, parsed_asts: list[dict]) -> ComputationSpec:
        if not parsed_asts:
            raise ValueError("ODE handler requires derivative expressions")

        params = numeric_parameters(step)
        ivps: list[dict] = []

        if step.concept_type == ConceptType.ODE_PHASE_PORTRAIT and step.domain is not None and step.domain.x and step.domain.y:
            x_axis = step.domain.x
            y_axis = step.domain.y
            seed_n = int(params.get("seed_grid", 12))
            seed_n = max(2, min(64, seed_n))
            for iy in range(seed_n):
                for ix in range(seed_n):
                    x0 = x_axis.min + (x_axis.max - x_axis.min) * (ix / max(1, seed_n - 1))
                    y0 = y_axis.min + (y_axis.max - y_axis.min) * (iy / max(1, seed_n - 1))
                    ivps.append(
                        {
                            "derivatives": parsed_asts[:2],
                            "initial_state": [x0, y0],
                            "t0": float(params.get("t0", 0.0)),
                            "t_end": float(params.get("t_end", 10.0)),
                            "method": "rk45",
                            "step_size": float(params.get("h", 0.02)),
                            "max_steps": int(params.get("max_steps", 3000)),
                            "layer_id": f"ode_{step.step_index}_{iy}_{ix}",
                        }
                    )
        else:
            initial = [0.0] * len(parsed_asts)
            for i, key in enumerate(("x0", "y0", "z0")):
                if key in params and i < len(initial):
                    initial[i] = float(params[key])

            ivps = [
                {
                    "derivatives": parsed_asts,
                    "initial_state": initial,
                    "t0": float(params.get("t0", 0.0)),
                    "t_end": float(params.get("t_end", 10.0)),
                    "method": "rk45",
                    "step_size": float(params.get("h", 0.02)),
                    "max_steps": int(params.get("max_steps", 5000)),
                    "layer_id": f"ode_{step.step_index}",
                }
            ]

        payload = {
            "ivps": ivps,
            "parameters": params,
            "allow_non_finite": False,
        }
        return ComputationSpec(rust_function_name="solve_ode_batch", request_payload=payload)

    def build_layer_metadata(self, step: StepDescriptor) -> list[LayerMetadata]:
        return layer_meta_list(step, [f"ode_{step.step_index}"])
