from __future__ import annotations

import hashlib
import json
from typing import Any

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


def layer_meta_list(step: StepDescriptor, layer_ids: list[str]) -> list[LayerMetadata]:
    return [default_layer_meta(step, layer_id) for layer_id in layer_ids]


def ensure_expression_list(step: StepDescriptor) -> list[str]:
    if step.expression is None:
        return []
    if isinstance(step.expression, str):
        return [step.expression]
    return step.expression


def numeric_parameters(step: StepDescriptor) -> dict[str, float]:
    return {k: float(v) for k, v in step.parameters.items() if isinstance(v, (int, float))}


def stable_hash(parts: dict[str, Any]) -> str:
    raw = json.dumps(parts, sort_keys=True, separators=(",", ":"))
    return hashlib.sha256(raw.encode("utf-8")).hexdigest()[:24]


def density_to_steps(density: str) -> int:
    m = {
        "low": 64,
        "medium": 256,
        "high": 512,
        "ultra": 1024,
    }
    return m.get(density.lower(), 256)
