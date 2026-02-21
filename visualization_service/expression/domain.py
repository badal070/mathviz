from __future__ import annotations

from typing import Any

import numpy as np

from visualization_service.schema.step_descriptor import DomainSpec


def build_domain(domain_spec: DomainSpec) -> dict[str, np.ndarray]:
    out: dict[str, np.ndarray] = {}
    for axis_name in ("x", "y", "z", "t"):
        axis = getattr(domain_spec, axis_name)
        if axis is None:
            continue
        out[axis_name] = np.linspace(axis.min, axis.max, axis.steps, dtype=np.float64)
    return out


def build_parameter_frames(parameters: dict[str, Any]) -> list[dict[str, float]]:
    animated = {k: v for k, v in parameters.items() if isinstance(v, dict) and "frames" in v}
    if not animated:
        return [{k: float(v) for k, v in parameters.items() if isinstance(v, (int, float))}]

    max_frames = max(int(v["frames"]) for v in animated.values())
    frames: list[dict[str, float]] = []
    for i in range(max_frames):
        frame: dict[str, float] = {}
        for key, value in parameters.items():
            if isinstance(value, (int, float)):
                frame[key] = float(value)
            elif isinstance(value, dict) and "frames" in value:
                fcount = max(1, int(value["frames"]))
                t = min(i, fcount - 1) / max(1, fcount - 1)
                frame[key] = float(value["min"]) + (float(value["max"]) - float(value["min"])) * t
        frames.append(frame)
    return frames
