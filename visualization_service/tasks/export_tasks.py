from __future__ import annotations

from collections import OrderedDict
from datetime import UTC, datetime
from pathlib import Path
import re
import socket
from typing import Any

import numpy as np
from redis import Redis

from visualization_service.config import settings
from visualization_service.geometry.serializer import deserialize_bundle
from visualization_service.tasks.celery_app import create_celery

celery_app = create_celery()

_SAFE_ID = re.compile(r"[^a-zA-Z0-9._-]+")
_RESOLUTION = re.compile(r"^\s*(\d{2,5})x(\d{2,5})\s*$")


class ExportError(RuntimeError):
    pass


def _require_pillow() -> tuple[Any, Any, Any]:
    try:
        from PIL import Image, ImageDraw, ImageFont
    except Exception as exc:  # noqa: BLE001
        raise ExportError("Pillow is required for export rendering") from exc
    return Image, ImageDraw, ImageFont


def _require_imageio() -> Any:
    try:
        import imageio.v2 as imageio
    except Exception as exc:  # noqa: BLE001
        raise ExportError("imageio with ffmpeg support is required for MP4 export") from exc
    return imageio


def _safe_scene_id(scene_id: str) -> str:
    clean = _SAFE_ID.sub("_", scene_id).strip("_")
    return clean or "scene"


def _parse_resolution(resolution: str | None) -> tuple[int, int]:
    if resolution is None:
        return settings.export_default_width, settings.export_default_height

    match = _RESOLUTION.match(resolution)
    if not match:
        raise ExportError(f"Invalid resolution '{resolution}'. Expected format WIDTHxHEIGHT.")

    width = max(320, min(3840, int(match.group(1))))
    height = max(240, min(2160, int(match.group(2))))
    return width, height


def _hex_to_rgb(value: Any) -> tuple[int, int, int]:
    if not isinstance(value, str):
        return (74, 144, 217)
    color = value.strip().lstrip("#")
    if len(color) != 6:
        return (74, 144, 217)
    try:
        return tuple(int(color[i : i + 2], 16) for i in (0, 2, 4))
    except ValueError:
        return (74, 144, 217)


def _as_vertices(buffer: Any) -> np.ndarray:
    if isinstance(buffer, (bytes, bytearray, memoryview)):
        arr = np.frombuffer(buffer, dtype=np.float32)
    elif isinstance(buffer, list):
        arr = np.asarray(buffer, dtype=np.float32)
    else:
        return np.empty((0, 3), dtype=np.float32)

    usable = (arr.size // 3) * 3
    if usable == 0:
        return np.empty((0, 3), dtype=np.float32)

    verts = arr[:usable].reshape(-1, 3)
    finite = np.isfinite(verts).all(axis=1)
    return verts[finite]


def _as_indices(buffer: Any) -> np.ndarray:
    if isinstance(buffer, (bytes, bytearray, memoryview)):
        return np.frombuffer(buffer, dtype=np.uint32)
    if isinstance(buffer, list):
        return np.asarray(buffer, dtype=np.uint32)
    return np.empty((0,), dtype=np.uint32)


def _load_scene_bundles(scene_id: str) -> list[dict[str, Any]]:
    redis = Redis.from_url(settings.redis_url, decode_responses=False)
    pattern = f"step_bundle:{scene_id}:*"

    try:
        key_parts: list[tuple[int, str]] = []
        for raw_key in redis.scan_iter(match=pattern, count=256):
            key = raw_key.decode("utf-8") if isinstance(raw_key, bytes) else str(raw_key)
            tail = key.rsplit(":", 1)[-1]
            if tail.isdigit():
                key_parts.append((int(tail), key))

        key_parts.sort(key=lambda item: item[0])
        if not key_parts:
            raise ExportError(f"No step bundles found for scene_id='{scene_id}'")

        if len(key_parts) > settings.export_max_frames:
            raise ExportError(
                f"Scene has {len(key_parts)} steps which exceeds export_max_frames={settings.export_max_frames}"
            )

        pipe = redis.pipeline(transaction=False)
        for _, key in key_parts:
            pipe.get(key)
        raw_bundles = pipe.execute()

        bundles: list[dict[str, Any]] = []
        for (step_index, _), raw_bundle in zip(key_parts, raw_bundles, strict=False):
            if not isinstance(raw_bundle, (bytes, bytearray, memoryview)):
                continue
            bundle = deserialize_bundle(bytes(raw_bundle))
            bundle["step_index"] = int(bundle.get("step_index", step_index))
            bundles.append(bundle)

        if not bundles:
            raise ExportError(f"Step bundles could not be decoded for scene_id='{scene_id}'")
        return bundles
    finally:
        redis.close()


def _reconstruct_frames(bundles: list[dict[str, Any]]) -> list[dict[str, Any]]:
    state: "OrderedDict[str, dict[str, Any]]" = OrderedDict()
    frames: list[dict[str, Any]] = []

    for bundle in sorted(bundles, key=lambda b: int(b.get("step_index", 0))):
        mode = str(bundle.get("layer_mode", "replace"))
        layers = bundle.get("layers", [])

        if mode in {"replace", "highlight"}:
            state = OrderedDict()
            for i, layer in enumerate(layers):
                layer_id = str(layer.get("layer_id", f"layer_{i}"))
                state[layer_id] = layer
        elif mode == "add":
            for i, layer in enumerate(layers):
                layer_id = str(layer.get("layer_id", f"layer_{i}"))
                if layer_id not in state:
                    state[layer_id] = layer

        frames.append(
            {
                "step_index": int(bundle.get("step_index", 0)),
                "step_label": str(bundle.get("step_label", "")),
                "hud_equation": str(bundle.get("hud_equation", "")),
                "narration": str(bundle.get("narration", "")),
                "layers": list(state.values()),
            }
        )

    return frames


def _project(points: np.ndarray, width: int, height: int, bounds: tuple[float, float, float, float]) -> np.ndarray:
    x_min, x_max, y_min, y_max = bounds
    x_center = (x_min + x_max) / 2.0
    y_center = (y_min + y_max) / 2.0

    dx = max(1e-6, x_max - x_min)
    dy = max(1e-6, y_max - y_min)
    margin = 0.08
    sx = ((1.0 - margin * 2.0) * width) / dx
    sy = ((1.0 - margin * 2.0) * height) / dy
    scale = min(sx, sy)

    px = (points[:, 0] - x_center) * scale + width / 2.0
    py = height / 2.0 - (points[:, 1] - y_center) * scale
    return np.stack([px, py], axis=1)


def _frame_bounds(layers: list[dict[str, Any]]) -> tuple[float, float, float, float]:
    samples: list[np.ndarray] = []
    for layer in layers:
        verts = _as_vertices(layer.get("vertex_buffer", b""))
        if verts.size:
            samples.append(verts[:, :2])

    if not samples:
        return (-1.0, 1.0, -1.0, 1.0)

    joined = np.concatenate(samples, axis=0)
    return (
        float(np.min(joined[:, 0])),
        float(np.max(joined[:, 0])),
        float(np.min(joined[:, 1])),
        float(np.max(joined[:, 1])),
    )


def _draw_layer(draw: Any, layer: dict[str, Any], width: int, height: int, bounds: tuple[float, float, float, float]) -> None:
    verts = _as_vertices(layer.get("vertex_buffer", b""))
    if verts.size == 0:
        return

    indices = _as_indices(layer.get("index_buffer", b""))
    projected = _project(verts, width, height, bounds)

    color = _hex_to_rgb(layer.get("color_hint"))
    opacity = float(layer.get("opacity", 1.0))
    opacity = max(0.05, min(1.0, opacity))
    rgba = (color[0], color[1], color[2], int(opacity * 255))

    max_segments = 20000

    if indices.size >= 3 and indices.size % 3 == 0:
        tris = indices.reshape(-1, 3)
        drawn = 0
        for tri in tris:
            if int(np.max(tri)) >= projected.shape[0]:
                continue
            p0 = tuple(projected[int(tri[0])])
            p1 = tuple(projected[int(tri[1])])
            p2 = tuple(projected[int(tri[2])])
            draw.line([p0, p1], fill=rgba, width=1)
            draw.line([p1, p2], fill=rgba, width=1)
            draw.line([p2, p0], fill=rgba, width=1)
            drawn += 3
            if drawn >= max_segments:
                break
        return

    if indices.size >= 2:
        drawn = 0
        pair_count = indices.size // 2
        for i in range(pair_count):
            a = int(indices[i * 2])
            b = int(indices[i * 2 + 1])
            if a >= projected.shape[0] or b >= projected.shape[0]:
                continue
            draw.line([tuple(projected[a]), tuple(projected[b])], fill=rgba, width=2)
            drawn += 1
            if drawn >= max_segments:
                break
        return

    if projected.shape[0] >= 2:
        stride = max(1, projected.shape[0] // 4096)
        points = projected[::stride]
        for i in range(points.shape[0] - 1):
            draw.line([tuple(points[i]), tuple(points[i + 1])], fill=rgba, width=2)
    else:
        p = tuple(projected[0])
        draw.ellipse((p[0] - 2, p[1] - 2, p[0] + 2, p[1] + 2), fill=rgba)


def _render_frames(frames: list[dict[str, Any]], width: int, height: int) -> list[Any]:
    Image, ImageDraw, ImageFont = _require_pillow()
    font = ImageFont.load_default()

    rendered: list[Any] = []
    for frame in frames:
        image = Image.new("RGBA", (width, height), (13, 18, 24, 255))
        draw = ImageDraw.Draw(image, "RGBA")

        # Subtle reference grid.
        grid_color = (40, 52, 66, 255)
        for gx in range(0, width, max(40, width // 20)):
            draw.line([(gx, 0), (gx, height)], fill=grid_color, width=1)
        for gy in range(0, height, max(40, height // 12)):
            draw.line([(0, gy), (width, gy)], fill=grid_color, width=1)

        bounds = _frame_bounds(frame["layers"])
        for layer in frame["layers"]:
            _draw_layer(draw, layer, width, height, bounds)

        draw.rectangle([(0, 0), (width, 82)], fill=(0, 0, 0, 155))
        draw.text((14, 10), f"Step {frame['step_index']}: {frame['step_label']}", font=font, fill=(240, 245, 250, 255))
        if frame["hud_equation"]:
            draw.text((14, 30), frame["hud_equation"], font=font, fill=(160, 235, 200, 255))
        if frame["narration"]:
            draw.text((14, 50), frame["narration"][:120], font=font, fill=(205, 215, 225, 255))

        rendered.append(image.convert("RGB"))

    return rendered


def _artifact_path(scene_id: str, suffix: str) -> Path:
    out_dir = Path(settings.export_output_dir)
    out_dir.mkdir(parents=True, exist_ok=True)
    stamp = datetime.now(UTC).strftime("%Y%m%dT%H%M%SZ")
    return out_dir / f"{_safe_scene_id(scene_id)}_{stamp}.{suffix}"


def _write_pdf(frames: list[Any], path: Path) -> None:
    if not frames:
        raise ExportError("No frames available for PDF export")
    first, rest = frames[0], frames[1:]
    first.save(path, format="PDF", resolution=144.0, save_all=True, append_images=rest)


def _write_gif(frames: list[Any], path: Path, fps: int) -> None:
    if not frames:
        raise ExportError("No frames available for GIF export")
    duration_ms = max(20, int(1000 / max(1, fps)))
    first, rest = frames[0], frames[1:]
    first.save(path, format="GIF", save_all=True, append_images=rest, duration=duration_ms, loop=0, disposal=2)


def _write_mp4(frames: list[Any], path: Path, fps: int) -> None:
    if not frames:
        raise ExportError("No frames available for MP4 export")

    imageio = _require_imageio()
    with imageio.get_writer(
        str(path),
        fps=max(1, fps),
        codec="libx264",
        quality=8,
        macro_block_size=None,
    ) as writer:
        for frame in frames:
            writer.append_data(np.asarray(frame))


def _export_common(
    *,
    scene_id: str,
    user_id: str,
    export_format: str,
    resolution: str | None = None,
    fps: int | None = None,
) -> dict[str, Any]:
    width, height = _parse_resolution(resolution)
    fps = max(1, min(60, int(fps or settings.export_default_fps)))

    if export_format == "mp4":
        if width % 2 != 0:
            width += 1
        if height % 2 != 0:
            height += 1

    bundles = _load_scene_bundles(scene_id)
    frames = _reconstruct_frames(bundles)
    rendered = _render_frames(frames, width, height)

    if export_format == "pdf":
        artifact = _artifact_path(scene_id, "pdf")
        _write_pdf(rendered, artifact)
        media_type = "application/pdf"
    elif export_format == "gif":
        artifact = _artifact_path(scene_id, "gif")
        _write_gif(rendered, artifact, fps)
        media_type = "image/gif"
    elif export_format == "mp4":
        artifact = _artifact_path(scene_id, "mp4")
        _write_mp4(rendered, artifact, fps)
        media_type = "video/mp4"
    else:
        raise ExportError(f"Unsupported export format '{export_format}'")

    return {
        "scene_id": scene_id,
        "user_id": user_id,
        "format": export_format,
        "status": "completed",
        "artifact_path": str(artifact),
        "artifact_name": artifact.name,
        "mime_type": media_type,
        "frame_count": len(rendered),
        "resolution": f"{width}x{height}",
        "fps": fps,
        "generated_at": datetime.now(UTC).isoformat(),
        "worker": socket.gethostname(),
    }


def export_pdf(scene_id: str, user_id: str, resolution: str | None = None) -> dict[str, Any]:
    return _export_common(scene_id=scene_id, user_id=user_id, export_format="pdf", resolution=resolution)


def export_video(scene_id: str, user_id: str, resolution: str = "1920x1080", fps: int | None = None) -> dict[str, Any]:
    return _export_common(
        scene_id=scene_id,
        user_id=user_id,
        export_format="mp4",
        resolution=resolution,
        fps=fps,
    )


def export_gif(scene_id: str, user_id: str, resolution: str | None = None, fps: int | None = None) -> dict[str, Any]:
    return _export_common(
        scene_id=scene_id,
        user_id=user_id,
        export_format="gif",
        resolution=resolution,
        fps=fps,
    )


if celery_app is not None:
    export_pdf = celery_app.task(name="export_pdf")(export_pdf)
    export_video = celery_app.task(name="export_video")(export_video)
    export_gif = celery_app.task(name="export_gif")(export_gif)
