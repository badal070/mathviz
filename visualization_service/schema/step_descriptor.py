from __future__ import annotations

from typing import Annotated

from pydantic import BaseModel, ConfigDict, Field, field_validator, model_validator

from visualization_service.schema.enums import ConceptType, LayerMode, TransitionType


class AxisSpec(BaseModel):
    model_config = ConfigDict(extra="forbid")

    min: float
    max: float
    steps: Annotated[int, Field(ge=64, le=2048)] = 256


class DomainSpec(BaseModel):
    model_config = ConfigDict(extra="forbid")

    x: AxisSpec | None = None
    y: AxisSpec | None = None
    z: AxisSpec | None = None
    t: AxisSpec | None = None

    @model_validator(mode="after")
    def validate_bounds(self) -> "DomainSpec":
        any_axis = False
        for axis_name in ("x", "y", "z", "t"):
            axis = getattr(self, axis_name)
            if axis is None:
                continue
            any_axis = True
            if not axis.min < axis.max:
                raise ValueError(f"{axis_name}.min must be < {axis_name}.max")
        if not any_axis:
            raise ValueError("at least one domain axis must be provided")
        return self

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
    model_config = ConfigDict(extra="forbid")

    color: str = "#4A90D9"
    opacity: Annotated[float, Field(ge=0.0, le=1.0)] = 1.0
    line_width: Annotated[float, Field(gt=0.0)] = 1.0
    surface_density: str = "medium"
    show_wireframe: bool = False
    show_normals: bool = False


class Annotation(BaseModel):
    model_config = ConfigDict(extra="forbid")

    position: dict[str, float]
    label: Annotated[str, Field(min_length=1, max_length=200)]
    offset: dict[str, float] = Field(default_factory=lambda: {"x": 0.0, "y": 0.0})
    style: str = "default"


class StepDescriptor(BaseModel):
    model_config = ConfigDict(extra="forbid")

    step_index: Annotated[int, Field(ge=1)]
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
        if isinstance(v, list):
            if not v:
                raise ValueError("expression list cannot be empty")
            if any(not x.strip() for x in v):
                raise ValueError("expression list entries cannot be empty")
        return v

    @model_validator(mode="after")
    def validate_concept_requirements(self) -> "StepDescriptor":
        matrix_types = {ConceptType.LINEAR_TRANSFORM, ConceptType.EIGENSPACE_TRANSFORM}
        if self.concept_type in matrix_types and self.expression is None:
            raise ValueError("matrix-based concepts require expression")
        if self.concept_type == ConceptType.IMPLICIT_SURFACE and self.domain is not None:
            if self.domain.z is None:
                raise ValueError("implicit_surface requires z axis in domain")
        return self
