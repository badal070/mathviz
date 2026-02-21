from __future__ import annotations

from pydantic import BaseModel, Field

from visualization_service.schema.enums import ConceptType, LayerMode, TransitionType
from visualization_service.schema.step_descriptor import Annotation


class LayerPayload(BaseModel):
    layer_id: str
    source_expression: str
    concept_type: ConceptType
    vertex_buffer: bytes = b""
    normal_buffer: bytes = b""
    index_buffer: bytes = b""
    uv_buffer: bytes = b""
    instance_buffer: bytes = b""
    color_hint: str = "#4A90D9"
    opacity: float = Field(default=1.0, ge=0.0, le=1.0)


class StepBundle(BaseModel):
    step_index: int
    step_label: str
    hud_equation: str
    narration: str
    layer_mode: LayerMode
    transition: TransitionType
    layers: list[LayerPayload]
    annotations: list[Annotation]
    is_delta: bool = False
