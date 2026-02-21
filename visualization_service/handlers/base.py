from __future__ import annotations

from abc import ABC, abstractmethod
from dataclasses import dataclass
from typing import Any

from visualization_service.schema.step_descriptor import RenderHints, StepDescriptor


@dataclass(frozen=True)
class ComputationSpec:
    rust_function_name: str
    request_payload: dict[str, Any]


@dataclass(frozen=True)
class LayerMetadata:
    layer_id: str
    source_expression: str
    concept_type: str
    color_hint: str
    opacity: float
    render_hints: RenderHints


class ConceptHandler(ABC):
    @abstractmethod
    def build_computation_spec(
        self,
        step: StepDescriptor,
        domain_arrays: dict[str, Any],
        parsed_asts: list[dict[str, Any]],
    ) -> ComputationSpec:
        raise NotImplementedError

    @abstractmethod
    def build_layer_metadata(self, step: StepDescriptor) -> list[LayerMetadata]:
        raise NotImplementedError
