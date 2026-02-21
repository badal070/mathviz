from __future__ import annotations

from visualization_service.handlers.base import LayerMetadata
from visualization_service.schema.step_descriptor import StepDescriptor


def default_layer_meta(step: StepDescriptor, layer_id: str) -> LayerMetadata:
    expr = step.hud_equation
    return LayerMetadata(
        layer_id=layer_id,
        source_expression=expr,
        concept_type=step.concept_type.value,
        color_hint=step.render_hints.color,
        opacity=step.render_hints.opacity,
        render_hints=step.render_hints,
    )


def ensure_expression_list(step: StepDescriptor) -> list[str]:
    if step.expression is None:
        return []
    if isinstance(step.expression, str):
        return [step.expression]
    return step.expression
