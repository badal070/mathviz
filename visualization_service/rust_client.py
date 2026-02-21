from __future__ import annotations

import importlib
import json
from dataclasses import dataclass
from typing import Any

from visualization_service.config import settings


class RustClientError(RuntimeError):
    pass


@dataclass(frozen=True)
class RustClient:
    _core: Any

    @classmethod
    def load(cls) -> "RustClient":
        try:
            core = importlib.import_module(settings.rust_module_name)
        except Exception as exc:  # noqa: BLE001
            raise RustClientError(f"Failed to import Rust module '{settings.rust_module_name}': {exc}") from exc
        return cls(_core=core)

    def healthcheck(self) -> bool:
        return hasattr(self._core, "batch_evaluate")

    def configure(self, num_threads: int) -> bool:
        if not hasattr(self._core, "configure"):
            return False
        return bool(self._core.configure(num_threads))

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
