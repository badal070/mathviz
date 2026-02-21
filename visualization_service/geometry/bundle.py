from __future__ import annotations

from typing import Any

from visualization_service.handlers.base import LayerMetadata
from visualization_service.schema.geometry_wire import LayerPayload, StepBundle
from visualization_service.schema.step_descriptor import StepDescriptor


def assemble_bundle(
    step: StepDescriptor,
    normalized_layers: list[dict[str, Any]],
    layer_meta: list[LayerMetadata],
    is_delta: bool,
) -> StepBundle:
    meta_by_id = {m.layer_id: m for m in layer_meta}
    default_meta = layer_meta[0] if layer_meta else None

    layers: list[LayerPayload] = []
    for layer in normalized_layers:
        layer_id = str(layer.get("layer_id", "layer"))
        meta = meta_by_id.get(layer_id, default_meta)

        source_expression = layer.get("source_expression") or (meta.source_expression if meta else step.hud_equation)
        color = layer.get("color_hint") or (meta.color_hint if meta else step.render_hints.color)
        opacity = float(layer.get("opacity", meta.opacity if meta else step.render_hints.opacity))

        layers.append(
            LayerPayload(
                layer_id=layer_id,
                source_expression=source_expression,
                concept_type=step.concept_type,
                vertex_buffer=_extract_bytes(layer.get("vertex_buffer")),
                normal_buffer=_extract_bytes(layer.get("normal_buffer")),
                index_buffer=_extract_bytes(layer.get("index_buffer"), is_index=True),
                uv_buffer=_extract_bytes(layer.get("uv_buffer")),
                instance_buffer=_extract_bytes(layer.get("instance_buffer")),
                color_hint=color,
                opacity=opacity,
            )
        )

    return StepBundle(
        step_index=step.step_index,
        step_label=step.step_label,
        hud_equation=step.hud_equation,
        narration=step.narration,
        layer_mode=step.layer_mode,
        transition=step.transition,
        layers=layers,
        annotations=step.annotations,
        is_delta=is_delta,
    )


def _extract_bytes(value: Any, *, is_index: bool = False) -> bytes:
    if value is None:
        return b""
    if isinstance(value, bytes):
        return value
    if hasattr(value, "tobytes"):
        return value.tobytes()
    if isinstance(value, list):
        import array

        if is_index:
            return array.array("I", [int(v) for v in value]).tobytes()
        return array.array("f", [float(v) for v in value]).tobytes()
    return b""
