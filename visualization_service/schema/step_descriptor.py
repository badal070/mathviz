from __future__ import annotations

from typing import Annotated

from pydantic import BaseModel, Field, field_validator

from visualization_service.schema.enums import ConceptType, LayerMode, TransitionType


class AxisSpec(BaseModel):
    min: float
    max: float
    steps: Annotated[int, Field(ge=64, le=2048)] = 256


class DomainSpec(BaseModel):
    x: AxisSpec | None = None
    y: AxisSpec | None = None
    z: AxisSpec | None = None
    t: AxisSpec | None = None

    def clamped(self) -> "DomainSpec":
        def _clamp_axis(axis: AxisSpec | None) -> AxisSpec | None:
            if axis is None:
                return None
            return AxisSpec(min=axis.min, max=axis.max, steps=max(64, min(2048, axis.steps)))

        return DomainSpec(
            x=_clamp_axis(self.x),
            y=_clamp_axis(self.y),
            z=_clamp_axis(self.z),
            t=_clamp_axis(self.t),
        )


class RenderHints(BaseModel):
    color: str = "#4A90D9"
    opacity: Annotated[float, Field(ge=0.0, le=1.0)] = 1.0
    line_width: float = 1.0
    surface_density: str = "medium"
    show_wireframe: bool = False
    show_normals: bool = False


class Annotation(BaseModel):
    position: dict[str, float]
    label: Annotated[str, Field(min_length=1, max_length=200)]
    offset: dict[str, float] = Field(default_factory=lambda: {"x": 0.0, "y": 0.0})
    style: str = "default"


class StepDescriptor(BaseModel):
    step_index: int
    step_label: Annotated[str, Field(min_length=1, max_length=200)]
    concept_type: ConceptType
    expression: str | list[str] | None = None
    narration: Annotated[str, Field(min_length=1, max_length=1000)]
    domain: DomainSpec | None = None
    parameters: dict[str, float | dict[str, float | int | bool]] = Field(default_factory=dict)
    layer_mode: LayerMode
    transition: TransitionType
    render_hints: RenderHints = Field(default_factory=RenderHints)
    hud_equation: Annotated[str, Field(min_length=1, max_length=500)]
    annotations: list[Annotation] = Field(default_factory=list)

    @field_validator("expression")
    @classmethod
    def validate_expression_presence(cls, v: str | list[str] | None) -> str | list[str] | None:
        if v is None:
            return v
        if isinstance(v, str) and not v.strip():
            raise ValueError("expression cannot be empty")
        if isinstance(v, list) and not v:
            raise ValueError("expression list cannot be empty")
        return v
