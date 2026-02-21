from __future__ import annotations

import asyncio
import contextlib
import logging
from dataclasses import dataclass
from typing import Any

from visualization_service.cache.geometry_cache import GeometryCache
from visualization_service.cache.session_cache import SceneState, SessionCache
from visualization_service.config import settings
from visualization_service.expression.cache_key import compute_cache_key
from visualization_service.expression.domain import build_domain
from visualization_service.expression.parser import ExpressionParseError, parse_expression
from visualization_service.expression.symbols import build_registry
from visualization_service.geometry.bundle import assemble_bundle
from visualization_service.geometry.delta import compute_delta_layers
from visualization_service.geometry.serializer import deserialize_bundle, serialize_bundle
from visualization_service.handlers import HANDLER_REGISTRY
from visualization_service.handlers.base import LayerMetadata
from visualization_service.rust_client import RustClient
from visualization_service.schema.enums import LayerMode
from visualization_service.schema.scene_description import SceneDescription
from visualization_service.sequencer.playback_events import JumpTo, PlaybackEvent, Reset, SetPlaying, SetSpeed, StepBack, StepForward
from visualization_service.sequencer.step_queue import StepQueue, StepQueueEntry

logger = logging.getLogger(__name__)


@dataclass
class SequencerOutput:
    bundle_bytes: bytes
    step_index: int


class StepSequencer:
    def __init__(
        self,
        scene: SceneDescription,
        geometry_cache: GeometryCache,
        session_cache: SessionCache,
        rust_client: RustClient,
    ) -> None:
        self.scene = scene
        self.geometry_cache = geometry_cache
        self.session_cache = session_cache
        self.rust = rust_client
        self.output_queue: asyncio.Queue[SequencerOutput] = asyncio.Queue()
        self._queue = self._build_queue()
        self._state = SceneState.new(str(scene.scene_id), self._queue.total_steps)
        self._running = False
        self._lock = asyncio.Lock()
        self._precompute_task: asyncio.Task[None] | None = None

    async def initialize(self) -> None:
        await self.session_cache.set_scene_state(str(self.scene.scene_id), self._state)
        # Prioritize step 1 for fast first-frame delivery.
        first = self._queue.get(1)
        await self._compute_step(first)
        self._precompute_task = asyncio.create_task(self._precompute_remaining(), name=f"precompute:{self.scene.scene_id}")

    async def start_streaming(self) -> None:
        self._running = True
        await self._emit_step(self._state.current_step)

        while self._running:
            state = await self.session_cache.get_scene_state(str(self.scene.scene_id))
            if state is None:
                await asyncio.sleep(0.05)
                continue
            self._state = state
            if not self._state.is_playing:
                await asyncio.sleep(0.05)
                continue

            await asyncio.sleep((settings.default_step_dwell_ms / 1000.0) / max(0.1, self._state.speed_multiplier))
            if self._state.current_step >= self._state.total_steps:
                self._state.is_playing = False
                await self.session_cache.set_scene_state(str(self.scene.scene_id), self._state)
                continue

            self._state.current_step += 1
            await self.session_cache.set_scene_state(str(self.scene.scene_id), self._state)
            await self._emit_step(self._state.current_step)

    async def stop(self) -> None:
        self._running = False
        if self._precompute_task is not None:
            self._precompute_task.cancel()
            with contextlib.suppress(asyncio.CancelledError):
                await self._precompute_task

    async def handle_event(self, event: PlaybackEvent) -> None:
        async with self._lock:
            if isinstance(event, StepForward):
                self._state.current_step = min(self._state.total_steps, self._state.current_step + 1)
                await self.session_cache.set_scene_state(str(self.scene.scene_id), self._state)
                await self._emit_step(self._state.current_step)
                return

            if isinstance(event, StepBack):
                self._state.current_step = max(1, self._state.current_step - 1)
                self._state.accumulated_layer_ids = []
                await self.session_cache.set_scene_state(str(self.scene.scene_id), self._state)
                await self._emit_step(self._state.current_step, force_full=True)
                return

            if isinstance(event, JumpTo):
                self._state.current_step = min(max(1, event.step), self._state.total_steps)
                self._state.accumulated_layer_ids = []
                await self.session_cache.set_scene_state(str(self.scene.scene_id), self._state)
                await self._emit_step(self._state.current_step, force_full=True)
                return

            if isinstance(event, Reset):
                self._state.current_step = 1
                self._state.accumulated_layer_ids = []
                await self.session_cache.set_scene_state(str(self.scene.scene_id), self._state)
                await self._emit_step(1, force_full=True)
                return

            if isinstance(event, SetSpeed):
                self._state.speed_multiplier = max(0.1, min(4.0, event.multiplier))
                await self.session_cache.set_scene_state(str(self.scene.scene_id), self._state)
                return

            if isinstance(event, SetPlaying):
                self._state.is_playing = event.value
                await self.session_cache.set_scene_state(str(self.scene.scene_id), self._state)
                return

    def _build_queue(self) -> StepQueue:
        entries: list[StepQueueEntry] = []
        for step in self.scene.steps:
            effective_step = step
            if step.domain is None and self.scene.global_domain is not None:
                effective_step = step.model_copy(update={"domain": self.scene.global_domain})

            expressions = (
                [effective_step.expression]
                if isinstance(effective_step.expression, str)
                else (effective_step.expression or [])
            )
            registry = build_registry({"x", "y", "z", "t"}, effective_step.parameters)

            parsed: list[dict[str, Any]] = []
            if effective_step.concept_type.value not in {"linear_transform", "eigenspace_transform"}:
                try:
                    parsed = [parse_expression(expr, registry) for expr in expressions]
                except ExpressionParseError:
                    logger.exception("Expression parsing failed for step %s", effective_step.step_index)
                    raise

            handler = HANDLER_REGISTRY[effective_step.concept_type]
            domain = effective_step.domain
            domain_arrays = build_domain(domain) if domain is not None else {}
            spec = handler.build_computation_spec(effective_step, domain_arrays, parsed)
            meta = handler.build_layer_metadata(effective_step)

            cache_key = compute_cache_key(
                parsed[0] if parsed else {"type": "literal", "value": 0.0},
                domain.model_dump(mode="json") if domain is not None else {},
            )

            entries.append(
                StepQueueEntry(
                    step_index=step.step_index,
                    computation_spec=spec,
                    layer_metadata=meta,
                    step_descriptor=effective_step,
                    bundle_cache_key=cache_key,
                )
            )

        return StepQueue(entries)

    async def _precompute_remaining(self) -> None:
        for entry in self._queue:
            if entry.step_index == 1:
                continue
            await self._compute_step(entry)

    async def _compute_step(self, entry: StepQueueEntry) -> None:
        entry.status = "computing"
        try:
            rust_result = self._dispatch_rust(entry.computation_spec)
            normalized = self._normalize_rust_result(entry, rust_result)
            bundle = assemble_bundle(entry.step_descriptor, normalized, entry.layer_metadata, is_delta=False)
            bundle_bytes = serialize_bundle(bundle)
            await self.geometry_cache.set_step_bundle(str(self.scene.scene_id), entry.step_index, bundle_bytes)
            entry.status = "ready"
        except Exception:  # noqa: BLE001
            entry.status = "error"
            logger.exception("Failed to precompute step %s", entry.step_index)

    def _dispatch_rust(self, spec) -> Any:
        fn = spec.rust_function_name
        payload = spec.request_payload
        if fn == "batch_evaluate":
            return self.rust.batch_evaluate(payload)
        if fn == "trace_curve":
            return self.rust.trace_curve(payload)
        if fn == "solve_ode_batch":
            return self.rust.solve_ode_batch(payload)
        if fn == "process_vector_field":
            return self.rust.process_vector_field(payload)
        if fn == "generate_riemann":
            return self.rust.generate_riemann(payload)
        if fn == "visualize_linear_transform":
            return self.rust.visualize_linear_transform(payload["matrix"], payload["domain"])
        raise ValueError(f"Unknown rust function: {fn}")

    def _normalize_rust_result(self, entry: StepQueueEntry, rust_result: Any) -> list[dict[str, Any]]:
        spec = entry.computation_spec
        fn = spec.rust_function_name
        payload = spec.request_payload

        if fn == "batch_evaluate":
            out: list[dict[str, Any]] = []
            for item in payload.get("entries", []):
                hash_key = item.get("hash_key")
                layer_id = item.get("layer_id", hash_key)
                result = rust_result.get(hash_key, {}) if isinstance(rust_result, dict) else {}
                geom = result.get("ok") if isinstance(result, dict) else None
                if geom is None and isinstance(result, dict):
                    geom = result
                if not isinstance(geom, dict):
                    geom = {}
                out.append(
                    {
                        "layer_id": layer_id,
                        "source_expression": entry.step_descriptor.hud_equation,
                        "vertex_buffer": geom.get("vertex_buffer", []),
                        "normal_buffer": geom.get("normal_buffer", []),
                        "index_buffer": geom.get("index_buffer", []),
                        "uv_buffer": geom.get("uv_buffer", []),
                    }
                )
            return out

        if fn == "trace_curve":
            geom = rust_result.get("geometry", rust_result) if isinstance(rust_result, dict) else {}
            return [
                {
                    "layer_id": f"curve_{entry.step_index}",
                    "source_expression": entry.step_descriptor.hud_equation,
                    "vertex_buffer": geom.get("vertex_buffer", []),
                    "normal_buffer": geom.get("normal_buffer", []),
                    "index_buffer": geom.get("index_buffer", []),
                    "uv_buffer": geom.get("uv_buffer", []),
                }
            ]

        if fn == "solve_ode_batch":
            out = []
            for i, traj in enumerate(rust_result if isinstance(rust_result, list) else []):
                state = traj.get("state", []) if isinstance(traj, dict) else []
                vertices = list(state)
                indices = list(range(max(0, len(vertices) // 3)))
                out.append(
                    {
                        "layer_id": traj.get("layer_id", f"ode_{entry.step_index}_{i}"),
                        "source_expression": entry.step_descriptor.hud_equation,
                        "vertex_buffer": vertices,
                        "index_buffer": indices,
                        "normal_buffer": [],
                        "uv_buffer": [],
                    }
                )
            return out

        if fn == "process_vector_field":
            out = []
            if isinstance(rust_result, dict):
                unit_arrow = rust_result.get("unit_arrow", {})
                out.append(
                    {
                        "layer_id": f"vector_{entry.step_index}_unit",
                        "source_expression": entry.step_descriptor.hud_equation,
                        "vertex_buffer": unit_arrow.get("vertex_buffer", []),
                        "normal_buffer": unit_arrow.get("normal_buffer", []),
                        "index_buffer": unit_arrow.get("index_buffer", []),
                        "uv_buffer": unit_arrow.get("uv_buffer", []),
                        "instance_buffer": rust_result.get("instance_buffer", []),
                    }
                )
                for i, line in enumerate(rust_result.get("streamlines", [])):
                    out.append(
                        {
                            "layer_id": line.get("layer_id", f"streamline_{i}"),
                            "source_expression": entry.step_descriptor.hud_equation,
                            "vertex_buffer": line.get("vertex_buffer", []),
                            "normal_buffer": line.get("normal_buffer", []),
                            "index_buffer": line.get("index_buffer", []),
                            "uv_buffer": line.get("uv_buffer", []),
                        }
                    )
            return out

        if fn == "generate_riemann":
            geom = rust_result if isinstance(rust_result, dict) else {}
            return [
                {
                    "layer_id": geom.get("layer_id", f"riemann_{entry.step_index}"),
                    "source_expression": entry.step_descriptor.hud_equation,
                    "vertex_buffer": geom.get("vertex_buffer", []),
                    "normal_buffer": geom.get("normal_buffer", []),
                    "index_buffer": geom.get("index_buffer", []),
                    "uv_buffer": geom.get("uv_buffer", []),
                }
            ]

        if fn == "visualize_linear_transform":
            out = []
            if isinstance(rust_result, dict):
                for key in ("before", "after"):
                    geom = rust_result.get(key, {})
                    out.append(
                        {
                            "layer_id": geom.get("layer_id", f"{key}_{entry.step_index}"),
                            "source_expression": entry.step_descriptor.hud_equation,
                            "vertex_buffer": geom.get("vertex_buffer", []),
                            "normal_buffer": geom.get("normal_buffer", []),
                            "index_buffer": geom.get("index_buffer", []),
                            "uv_buffer": geom.get("uv_buffer", []),
                        }
                    )
                for i, geom in enumerate(rust_result.get("eigen_layers", [])):
                    out.append(
                        {
                            "layer_id": geom.get("layer_id", f"eigen_{entry.step_index}_{i}"),
                            "source_expression": entry.step_descriptor.hud_equation,
                            "vertex_buffer": geom.get("vertex_buffer", []),
                            "normal_buffer": geom.get("normal_buffer", []),
                            "index_buffer": geom.get("index_buffer", []),
                            "uv_buffer": geom.get("uv_buffer", []),
                        }
                    )
                for i, geom in enumerate(rust_result.get("svd_layers", [])):
                    out.append(
                        {
                            "layer_id": geom.get("layer_id", f"svd_{entry.step_index}_{i}"),
                            "source_expression": entry.step_descriptor.hud_equation,
                            "vertex_buffer": geom.get("vertex_buffer", []),
                            "normal_buffer": geom.get("normal_buffer", []),
                            "index_buffer": geom.get("index_buffer", []),
                            "uv_buffer": geom.get("uv_buffer", []),
                        }
                    )
            return out

        return []

    async def _emit_step(self, step_index: int, force_full: bool = False) -> None:
        raw = await self._wait_for_bundle(step_index)
        if raw is None:
            # Control message: buffering in progress.
            ctrl = {"v": 1, "type": "buffer", "step": step_index}
            await self.output_queue.put(SequencerOutput(bundle_bytes=serialize_json_like(ctrl), step_index=step_index))
            return

        if force_full:
            bundle_dict = await self._build_replay_bundle(step_index)
        else:
            bundle_dict = deserialize_bundle(raw)

        layer_mode = bundle_dict.get("layer_mode")
        layers = bundle_dict.get("layers", [])

        if force_full:
            bundle_dict["is_delta"] = False
            self._state.accumulated_layer_ids = [str(l.get("layer_id")) for l in layers]
        elif layer_mode == LayerMode.ADD.value:
            existing = set(self._state.accumulated_layer_ids)
            filtered = [l for l in layers if str(l.get("layer_id")) not in existing]
            for l in filtered:
                self._state.accumulated_layer_ids.append(str(l.get("layer_id")))
            bundle_dict["layers"] = filtered
            bundle_dict["is_delta"] = True
        elif layer_mode == LayerMode.REPLACE.value:
            self._state.accumulated_layer_ids = [str(l.get("layer_id")) for l in layers]
            bundle_dict["is_delta"] = False
        else:  # highlight
            bundle_dict["is_delta"] = False

        await self.session_cache.set_scene_state(str(self.scene.scene_id), self._state)
        await self.output_queue.put(
            SequencerOutput(
                bundle_bytes=serialize_json_like(bundle_dict),
                step_index=step_index,
            )
        )

    async def _wait_for_bundle(self, step_index: int) -> bytes | None:
        for _ in range(20):
            raw = await self.geometry_cache.get_step_bundle(str(self.scene.scene_id), step_index)
            if raw is not None:
                return raw
            await asyncio.sleep(0.01)
        return None

    async def _build_replay_bundle(self, target_step: int) -> dict[str, Any]:
        replay_layers: list[dict[str, Any]] = []
        accumulated: set[str] = set()
        target_bundle: dict[str, Any] | None = None

        for idx in range(1, target_step + 1):
            raw = await self.geometry_cache.get_step_bundle(str(self.scene.scene_id), idx)
            if raw is None:
                continue
            bundle = deserialize_bundle(raw)
            mode = bundle.get("layer_mode")
            layers = bundle.get("layers", [])

            if mode == LayerMode.REPLACE.value:
                replay_layers = list(layers)
                accumulated = {str(l.get("layer_id")) for l in replay_layers}
            elif mode == LayerMode.ADD.value:
                for layer in layers:
                    lid = str(layer.get("layer_id"))
                    if lid not in accumulated:
                        replay_layers.append(layer)
                        accumulated.add(lid)
            else:  # highlight
                replay_layers = list(layers)
                accumulated = {str(l.get("layer_id")) for l in replay_layers}

            if idx == target_step:
                target_bundle = bundle

        if target_bundle is None:
            target_bundle = {
                "v": 1,
                "step_index": target_step,
                "step_label": "",
                "hud_equation": "",
                "narration": "",
                "layer_mode": LayerMode.REPLACE.value,
                "transition": "fade_in",
                "is_delta": False,
                "annotations": [],
                "layers": [],
            }

        target_bundle["layers"] = replay_layers
        target_bundle["is_delta"] = False
        return target_bundle


def serialize_json_like(payload: dict[str, Any]) -> bytes:
    import msgpack

    return msgpack.packb(payload, use_bin_type=True)
