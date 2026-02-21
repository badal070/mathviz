from __future__ import annotations

from typing import Annotated
from uuid import UUID

from pydantic import BaseModel, ConfigDict, Field, model_validator

from visualization_service.schema.enums import CoordinateSystem, LayerMode
from visualization_service.schema.step_descriptor import DomainSpec, RenderHints, StepDescriptor


class SceneDescription(BaseModel):
    model_config = ConfigDict(extra="forbid")

    scene_id: UUID
    version: str = "2.0"
    session_id: UUID
    concept_title: Annotated[str, Field(min_length=1, max_length=300)]
    concept_summary: Annotated[str, Field(min_length=1, max_length=1000)]
    total_steps: Annotated[int, Field(ge=1, le=64)]
    default_camera: str | dict[str, float] | None = None
    coordinate_system: CoordinateSystem = CoordinateSystem.CARTESIAN
    steps: list[StepDescriptor]
    global_domain: DomainSpec | None = None
    global_render_hints: RenderHints | None = None

    @model_validator(mode="after")
    def validate_steps(self) -> "SceneDescription":
        if self.total_steps != len(self.steps):
            raise ValueError("total_steps must equal len(steps)")

        expected = 1
        for step in self.steps:
            if step.step_index != expected:
                raise ValueError("step indices must be contiguous and start at 1")
            expected += 1

        if self.steps and self.steps[0].layer_mode == LayerMode.HIGHLIGHT:
            raise ValueError("step 1 cannot use highlight layer_mode")

        return self
