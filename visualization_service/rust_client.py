from __future__ import annotations

import importlib
import json
from dataclasses import dataclass
from typing import Any


@dataclass(frozen=True)
class RustClient:
    _core: Any

    @classmethod
    def load(cls) -> "RustClient":
        core = importlib.import_module("mathviz_core")
        return cls(_core=core)

    def batch_evaluate(self, payload: dict[str, Any]) -> Any:
        return self._core.batch_evaluate(json.dumps(payload))

    def trace_curve(self, payload: dict[str, Any]) -> Any:
        return self._core.trace_curve(json.dumps(payload))

    def solve_ode_batch(self, payload: dict[str, Any]) -> Any:
        return self._core.solve_ode_batch(json.dumps(payload))

    def process_vector_field(self, payload: dict[str, Any]) -> Any:
        return self._core.process_vector_field(json.dumps(payload))

    def generate_riemann(self, payload: dict[str, Any]) -> Any:
        return self._core.generate_riemann(json.dumps(payload))

    def visualize_linear_transform(self, matrix: Any, domain: dict[str, Any]) -> Any:
        return self._core.visualize_linear_transform(json.dumps(matrix), json.dumps(domain))
