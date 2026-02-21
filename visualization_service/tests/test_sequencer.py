from uuid import uuid4

import pytest

from visualization_service.cache.geometry_cache import GeometryCache
from visualization_service.cache.session_cache import SessionCache
from visualization_service.rust_client import RustClient
from visualization_service.schema.scene_description import SceneDescription
from visualization_service.sequencer.playback_events import JumpTo, Reset, StepForward
from visualization_service.sequencer.step_sequencer import StepSequencer


class DummyRedis:
    def __init__(self):
        self.db = {}

    async def get(self, key):
        return self.db.get(key)

    async def setex(self, key, ttl, value):
        _ = ttl
        self.db[key] = value


class DummyCore:
    def batch_evaluate(self, payload):
        _ = payload
        return {"ok": {"vertex_buffer": [0.0, 0.0, 0.0], "normal_buffer": [], "index_buffer": [0], "uv_buffer": []}}

    trace_curve = batch_evaluate
    solve_ode_batch = batch_evaluate
    process_vector_field = batch_evaluate
    generate_riemann = batch_evaluate

    def visualize_linear_transform(self, matrix, domain):
        _ = matrix, domain
        return {"vertex_buffer": [0.0, 0.0, 0.0], "index_buffer": [0]}


@pytest.mark.asyncio
async def test_sequencer_events() -> None:
    raw = {
        "scene_id": str(uuid4()),
        "session_id": str(uuid4()),
        "concept_title": "Test",
        "concept_summary": "Summary",
        "total_steps": 1,
        "steps": [
            {
                "step_index": 1,
                "step_label": "s1",
                "concept_type": "function_2d",
                "expression": "x",
                "narration": "n",
                "domain": {"x": {"min": -1, "max": 1, "steps": 128}},
                "layer_mode": "replace",
                "transition": "fade_in",
                "hud_equation": "x",
            }
        ],
    }
    scene = SceneDescription.model_validate(raw)
    redis = DummyRedis()
    g = GeometryCache(redis, 10, 10)
    s = SessionCache(redis, 10)
    sequencer = StepSequencer(scene, g, s, RustClient(DummyCore()))
    await sequencer.initialize()

    await sequencer.handle_event(StepForward())
    await sequencer.handle_event(JumpTo(step=1))
    await sequencer.handle_event(Reset())

    state = await s.get_scene_state(str(scene.scene_id))
    assert state is not None
    assert state.current_step == 1
