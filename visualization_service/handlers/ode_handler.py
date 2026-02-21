from __future__ import annotations

from visualization_service.handlers._common import default_layer_meta
from visualization_service.handlers.base import ComputationSpec, ConceptHandler, LayerMetadata
from visualization_service.schema.step_descriptor import StepDescriptor


class ODEHandler(ConceptHandler):
    def build_computation_spec(self, step: StepDescriptor, domain_arrays: dict, parsed_asts: list[dict]) -> ComputationSpec:
        if not parsed_asts:
            raise ValueError("ODE handler requires derivative expressions")

        initial = [0.0] * len(parsed_asts)
        params = {k: v for k, v in step.parameters.items() if isinstance(v, (int, float))}
        for i, key in enumerate(("x0", "y0", "z0")):
            if key in params and i < len(initial):
                initial[i] = float(params[key])

        ivp = {
            "derivatives": parsed_asts,
            "initial_state": initial,
            "t0": float(params.get("t0", 0.0)),
            "t_end": float(params.get("t_end", 10.0)),
            "method": "rk45",
            "step_size": float(params.get("h", 0.02)),
            "layer_id": f"ode_{step.step_index}",
        }
        payload = {"ivps": [ivp], "parameters": params}
        return ComputationSpec(rust_function_name="solve_ode_batch", request_payload=payload)

    def build_layer_metadata(self, step: StepDescriptor) -> LayerMetadata:
        return default_layer_meta(step, f"ode_{step.step_index}")
