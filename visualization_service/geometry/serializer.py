from __future__ import annotations

from typing import Any

import msgpack

from visualization_service.schema.geometry_wire import StepBundle


BUNDLE_VERSION = 1


def serialize_bundle(bundle: StepBundle) -> bytes:
    payload = {
        "v": BUNDLE_VERSION,
        "step_index": bundle.step_index,
        "step_label": bundle.step_label,
        "hud_equation": bundle.hud_equation,
        "narration": bundle.narration,
        "layer_mode": bundle.layer_mode.value,
        "transition": bundle.transition.value,
        "is_delta": bundle.is_delta,
        "annotations": [a.model_dump(mode="json") for a in bundle.annotations],
        "layers": [
            {
                "layer_id": l.layer_id,
                "source_expression": l.source_expression,
                "concept_type": l.concept_type.value,
                "vertex_buffer": l.vertex_buffer,
                "normal_buffer": l.normal_buffer,
                "index_buffer": l.index_buffer,
                "uv_buffer": l.uv_buffer,
                "instance_buffer": l.instance_buffer,
                "color_hint": l.color_hint,
                "opacity": l.opacity,
            }
            for l in bundle.layers
        ],
    }
    return msgpack.packb(payload, use_bin_type=True)


def deserialize_bundle(bundle_bytes: bytes) -> dict[str, Any]:
    return msgpack.unpackb(bundle_bytes, raw=False)
