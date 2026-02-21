from __future__ import annotations

from dataclasses import dataclass
from typing import Literal

from visualization_service.handlers.base import ComputationSpec, LayerMetadata
from visualization_service.schema.step_descriptor import StepDescriptor


@dataclass
class StepQueueEntry:
    step_index: int
    computation_spec: ComputationSpec
    layer_metadata: list[LayerMetadata]
    step_descriptor: StepDescriptor
    bundle_cache_key: str
    status: Literal["pending", "computing", "ready", "error"] = "pending"


class StepQueue:
    def __init__(self, entries: list[StepQueueEntry]) -> None:
        self._entries = sorted(entries, key=lambda e: e.step_index)

    def __iter__(self):
        return iter(self._entries)

    def get(self, step_index: int) -> StepQueueEntry:
        for entry in self._entries:
            if entry.step_index == step_index:
                return entry
        raise KeyError(step_index)

    @property
    def total_steps(self) -> int:
        return len(self._entries)
