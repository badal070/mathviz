from __future__ import annotations

from fastapi import APIRouter, Depends, HTTPException, WebSocket

from visualization_service.api.websocket_manager import WebSocketManager
from visualization_service.cache.geometry_cache import GeometryCache
from visualization_service.cache.session_cache import SceneState, SessionCache
from visualization_service.dependencies import geometry_cache_dep, session_cache_dep
from visualization_service.rust_client import RustClient
from visualization_service.schema.scene_description import SceneDescription
from visualization_service.sequencer.step_sequencer import StepSequencer

router = APIRouter()
ws_manager = WebSocketManager()

_SEQUENCERS: dict[str, StepSequencer] = {}


@router.post("/scenes")
async def create_scene(
    scene: SceneDescription,
    geometry_cache: GeometryCache = Depends(geometry_cache_dep),
    session_cache: SessionCache = Depends(session_cache_dep),
) -> dict:
    try:
        rust = RustClient.load()
    except Exception as exc:  # noqa: BLE001
        raise HTTPException(status_code=503, detail=f"mathviz_core unavailable: {exc}") from exc
    sequencer = StepSequencer(scene, geometry_cache, session_cache, rust)
    await sequencer.initialize()
    _SEQUENCERS[str(scene.scene_id)] = sequencer
    return {
        "scene_id": str(scene.scene_id),
        "ws_url": f"/scenes/{scene.scene_id}/stream",
    }


@router.get("/scenes/{scene_id}/steps/{step_index}")
async def get_step_bundle(scene_id: str, step_index: int, geometry_cache: GeometryCache = Depends(geometry_cache_dep)):
    bundle = await geometry_cache.get_step_bundle(scene_id, step_index)
    if bundle is None:
        raise HTTPException(status_code=404, detail="step bundle not found")
    return {"step_index": step_index, "bytes": bundle.hex()}


@router.get("/scenes/{scene_id}/state")
async def get_scene_state(scene_id: str, session_cache: SessionCache = Depends(session_cache_dep)) -> SceneState:
    state = await session_cache.get_scene_state(scene_id)
    if state is None:
        raise HTTPException(status_code=404, detail="scene state not found")
    return state


@router.post("/scenes/{scene_id}/reset")
async def reset_scene(scene_id: str, session_cache: SessionCache = Depends(session_cache_dep)) -> dict:
    state = await session_cache.get_scene_state(scene_id)
    if state is None:
        raise HTTPException(status_code=404, detail="scene state not found")
    state.current_step = 1
    state.accumulated_layer_ids = []
    await session_cache.set_scene_state(scene_id, state)
    return {"ok": True}


@router.get("/health")
async def health(geometry_cache: GeometryCache = Depends(geometry_cache_dep)) -> dict:
    try:
        await geometry_cache.set_geometry("health", b"ok", ttl=1)
    except Exception as exc:  # noqa: BLE001
        raise HTTPException(status_code=503, detail=f"redis unavailable: {exc}") from exc
    return {"status": "ok"}


@router.websocket("/scenes/{scene_id}/stream")
async def stream_scene(scene_id: str, websocket: WebSocket) -> None:
    sequencer = _SEQUENCERS.get(scene_id)
    if sequencer is None:
        await websocket.close(code=4404)
        return
    await ws_manager.connect(scene_id, websocket, sequencer)
