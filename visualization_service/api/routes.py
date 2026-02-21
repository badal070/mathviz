from __future__ import annotations

import asyncio
import logging
from dataclasses import asdict

from fastapi import APIRouter, Depends, HTTPException, Response, WebSocket
from pydantic import BaseModel, Field

from visualization_service.api.auth import issue_ws_token
from visualization_service.api.websocket_manager import WebSocketManager
from visualization_service.cache.geometry_cache import GeometryCache
from visualization_service.cache.session_cache import SessionCache
from visualization_service.dependencies import geometry_cache_dep, rust_client_dep, session_cache_dep
from visualization_service.rust_client import RustClient
from visualization_service.schema.scene_description import SceneDescription
from visualization_service.sequencer.step_sequencer import StepSequencer
from visualization_service.tasks.celery_app import enqueue_task

logger = logging.getLogger(__name__)

router = APIRouter()
ws_manager = WebSocketManager()

_SEQUENCERS: dict[str, StepSequencer] = {}
_SCENE_LOCK = asyncio.Lock()


class ExportRequest(BaseModel):
    user_id: str = Field(min_length=1, max_length=128)
    resolution: str | None = None
    fps: int | None = Field(default=None, ge=1, le=60)


@router.post("/scenes")
async def create_scene(
    scene: SceneDescription,
    geometry_cache: GeometryCache = Depends(geometry_cache_dep),
    session_cache: SessionCache = Depends(session_cache_dep),
    rust: RustClient = Depends(rust_client_dep),
) -> dict:
    if not rust.healthcheck():
        raise HTTPException(status_code=503, detail="Rust core is unavailable")

    sequencer = StepSequencer(scene, geometry_cache, session_cache, rust)
    await sequencer.initialize()

    async with _SCENE_LOCK:
        _SEQUENCERS[str(scene.scene_id)] = sequencer

    ws_token = issue_ws_token(user_id="internal", session_id=str(scene.session_id), scene_id=str(scene.scene_id))
    return {
        "scene_id": str(scene.scene_id),
        "ws_url": f"/scenes/{scene.scene_id}/stream?token={ws_token}",
        "step_count": scene.total_steps,
    }


@router.get("/scenes/{scene_id}/steps/{step_index}")
async def get_step_bundle(
    scene_id: str,
    step_index: int,
    geometry_cache: GeometryCache = Depends(geometry_cache_dep),
) -> Response:
    bundle = await geometry_cache.get_step_bundle(scene_id, step_index)
    if bundle is None:
        raise HTTPException(status_code=404, detail="step bundle not found")
    return Response(content=bundle, media_type="application/msgpack")


@router.get("/scenes/{scene_id}/state")
async def get_scene_state(scene_id: str, session_cache: SessionCache = Depends(session_cache_dep)) -> dict:
    state = await session_cache.get_scene_state(scene_id)
    if state is None:
        raise HTTPException(status_code=404, detail="scene state not found")
    return asdict(state)


@router.post("/scenes/{scene_id}/reset")
async def reset_scene(scene_id: str, session_cache: SessionCache = Depends(session_cache_dep)) -> dict:
    state = await session_cache.get_scene_state(scene_id)
    if state is None:
        raise HTTPException(status_code=404, detail="scene state not found")

    state.current_step = 1
    state.accumulated_layer_ids = []
    await session_cache.set_scene_state(scene_id, state)
    return {"ok": True}


@router.post("/scenes/{scene_id}/exports/pdf")
async def queue_pdf_export(scene_id: str, req: ExportRequest) -> dict:
    try:
        task_id = enqueue_task(
            "export_pdf",
            kwargs={
                "scene_id": scene_id,
                "user_id": req.user_id,
                "resolution": req.resolution,
            },
        )
    except RuntimeError as exc:
        raise HTTPException(status_code=503, detail=str(exc)) from exc

    return {"status": "queued", "task_id": task_id, "format": "pdf"}


@router.post("/scenes/{scene_id}/exports/mp4")
async def queue_mp4_export(scene_id: str, req: ExportRequest) -> dict:
    try:
        task_id = enqueue_task(
            "export_video",
            kwargs={
                "scene_id": scene_id,
                "user_id": req.user_id,
                "resolution": req.resolution or "1920x1080",
                "fps": req.fps,
            },
        )
    except RuntimeError as exc:
        raise HTTPException(status_code=503, detail=str(exc)) from exc

    return {"status": "queued", "task_id": task_id, "format": "mp4"}


@router.post("/scenes/{scene_id}/exports/gif")
async def queue_gif_export(scene_id: str, req: ExportRequest) -> dict:
    try:
        task_id = enqueue_task(
            "export_gif",
            kwargs={
                "scene_id": scene_id,
                "user_id": req.user_id,
                "resolution": req.resolution,
                "fps": req.fps,
            },
        )
    except RuntimeError as exc:
        raise HTTPException(status_code=503, detail=str(exc)) from exc

    return {"status": "queued", "task_id": task_id, "format": "gif"}


@router.get("/health")
async def health(
    geometry_cache: GeometryCache = Depends(geometry_cache_dep),
    rust: RustClient = Depends(rust_client_dep),
) -> dict:
    redis_ok = True
    try:
        await geometry_cache.set_geometry("health", b"ok", ttl=1)
    except Exception as exc:  # noqa: BLE001
        redis_ok = False
        logger.warning("Redis health check failed: %s", exc)

    rust_ok = rust.healthcheck()
    if not redis_ok or not rust_ok:
        raise HTTPException(status_code=503, detail={"redis": redis_ok, "rust": rust_ok})

    return {"status": "ok", "redis": True, "rust": True}


@router.websocket("/scenes/{scene_id}/stream")
async def stream_scene(scene_id: str, websocket: WebSocket) -> None:
    sequencer = _SEQUENCERS.get(scene_id)
    if sequencer is None:
        await websocket.close(code=4404)
        return
    await ws_manager.connect(scene_id, websocket, sequencer)
