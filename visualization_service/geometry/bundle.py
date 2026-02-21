from __future__ import annotations

from typing import Any

from visualization_service.handlers.base import LayerMetadata
from visualization_service.schema.geometry_wire import LayerPayload, StepBundle
from visualization_service.schema.step_descriptor import StepDescriptor


def assemble_bundle(
    step: StepDescriptor,
    rust_results: dict[str, Any],
    layer_meta: list[LayerMetadata],
    is_delta: bool,
) -> StepBundle:
    layers: list[LayerPayload] = []

    for meta in layer_meta:
        geom = rust_results.get(meta.layer_id) or rust_results.get("ok") or rust_results

        layers.append(
            LayerPayload(
                layer_id=meta.layer_id,
                source_expression=meta.source_expression,
                concept_type=step.concept_type,
                vertex_buffer=_extract_bytes(geom, "vertex_buffer"),
                normal_buffer=_extract_bytes(geom, "normal_buffer"),
                index_buffer=_extract_bytes(geom, "index_buffer"),
                uv_buffer=_extract_bytes(geom, "uv_buffer"),
                instance_buffer=_extract_bytes(geom, "instance_buffer"),
                color_hint=meta.color_hint,
                opacity=meta.opacity,
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


def _extract_bytes(geom: Any, key: str) -> bytes:
    if isinstance(geom, dict):
        value = geom.get(key, b"")
    else:
        value = getattr(geom, key, b"")

    if value is None:
        return b""
    if isinstance(value, bytes):
        return value
    if hasattr(value, "tobytes"):
        return value.tobytes()
    if isinstance(value, list):
        import array

        if key == "index_buffer":
            return array.array("I", value).tobytes()
        return array.array("f", value).tobytes()
    return b""
