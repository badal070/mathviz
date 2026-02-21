from __future__ import annotations

import asyncio
from dataclasses import dataclass
from typing import Any

from visualization_service.cache.geometry_cache import GeometryCache
from visualization_service.cache.session_cache import SceneState, SessionCache
from visualization_service.config import settings
from visualization_service.expression.cache_key import compute_cache_key
from visualization_service.expression.domain import build_domain
from visualization_service.expression.parser import parse_expression
from visualization_service.expression.symbols import build_registry
from visualization_service.geometry.bundle import assemble_bundle
from visualization_service.geometry.delta import compute_delta_layers
from visualization_service.geometry.serializer import serialize_bundle
from visualization_service.handlers import HANDLER_REGISTRY
from visualization_service.rust_client import RustClient
from visualization_service.schema.enums import LayerMode
from visualization_service.schema.scene_description import SceneDescription
from visualization_service.sequencer.playback_events import JumpTo, PlaybackEvent, Reset, SetPlaying, SetSpeed, StepBack, StepForward
from visualization_service.sequencer.step_queue import StepQueue, StepQueueEntry


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

    async def initialize(self) -> None:
        await self.session_cache.set_scene_state(str(self.scene.scene_id), self._state)
        await self._precompute_steps()

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

    async def handle_event(self, event: PlaybackEvent) -> None:
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
            expressions = [step.expression] if isinstance(step.expression, str) else (step.expression or [])
            var_set = {"x", "y", "z", "t"}
            registry = build_registry(var_set, step.parameters)
            parsed: list[dict[str, Any]]
            if step.concept_type.value in {"linear_transform", "eigenspace_transform"}:
                parsed = []
            else:
                parsed = [parse_expression(expr, registry) for expr in expressions]
            handler = HANDLER_REGISTRY[step.concept_type]
            domain_arrays = build_domain(step.domain or self.scene.global_domain) if (step.domain or self.scene.global_domain) else {}
            spec = handler.build_computation_spec(step, domain_arrays, parsed)
            meta = handler.build_layer_metadata(step)
            cache_key = compute_cache_key(parsed[0] if parsed else {"type": "literal", "value": 0.0}, (step.domain or self.scene.global_domain).model_dump(mode="json") if (step.domain or self.scene.global_domain) else {})
            entries.append(
                StepQueueEntry(
                    step_index=step.step_index,
                    computation_spec=spec,
                    layer_metadata=meta,
                    step_descriptor=step,
                    bundle_cache_key=cache_key,
                )
            )
        return StepQueue(entries)

    async def _precompute_steps(self) -> None:
        for entry in self._queue:
            entry.status = "computing"
            rust_result = self._dispatch_rust(entry.computation_spec)
            step = entry.step_descriptor
            is_delta = step.layer_mode == LayerMode.ADD
            bundle = assemble_bundle(step, rust_result, [entry.layer_metadata], is_delta=is_delta)
            if step.layer_mode == LayerMode.ADD:
                bundle.layers = compute_delta_layers(bundle.layers, set(self._state.accumulated_layer_ids))
            bundle_bytes = serialize_bundle(bundle)
            await self.geometry_cache.set_step_bundle(str(self.scene.scene_id), step.step_index, bundle_bytes)
            entry.status = "ready"

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

    async def _emit_step(self, step_index: int, force_full: bool = False) -> None:
        raw = await self.geometry_cache.get_step_bundle(str(self.scene.scene_id), step_index)
        if raw is None:
            await asyncio.sleep(0.2)
            raw = await self.geometry_cache.get_step_bundle(str(self.scene.scene_id), step_index)
            if raw is None:
                return
        await self.output_queue.put(SequencerOutput(bundle_bytes=raw, step_index=step_index))
        if force_full:
            self._state.accumulated_layer_ids = []
