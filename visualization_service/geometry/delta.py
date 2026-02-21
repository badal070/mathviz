from __future__ import annotations

from visualization_service.schema.geometry_wire import LayerPayload


def compute_delta_layers(all_layers: list[LayerPayload], accumulated_ids: set[str]) -> list[LayerPayload]:
    out: list[LayerPayload] = []
    for layer in all_layers:
        if layer.layer_id not in accumulated_ids:
            out.append(layer)
            accumulated_ids.add(layer.layer_id)
    return out
