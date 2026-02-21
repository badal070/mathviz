from __future__ import annotations

import asyncio
import json
import logging

from fastapi import WebSocket, WebSocketDisconnect

from visualization_service.api.auth import AuthError, verify_ws_token
from visualization_service.sequencer.playback_events import PLAYBACK_ADAPTER
from visualization_service.sequencer.step_sequencer import StepSequencer

logger = logging.getLogger(__name__)


class WebSocketManager:
    def __init__(self) -> None:
        self._connections: dict[str, set[WebSocket]] = {}

    async def connect(self, scene_id: str, websocket: WebSocket, sequencer: StepSequencer) -> None:
        try:
            verify_ws_token(websocket, scene_id)
        except AuthError:
            await websocket.close(code=4008)
            return

        await websocket.accept()
        self._connections.setdefault(scene_id, set()).add(websocket)

        receiver_task = asyncio.create_task(self._receive_events(websocket, sequencer), name=f"ws-recv:{scene_id}")
        streamer_task = asyncio.create_task(self._stream_bundles(websocket, sequencer), name=f"ws-send:{scene_id}")
        sequencer_task = asyncio.create_task(sequencer.start_streaming(), name=f"seq:{scene_id}")

        done, pending = await asyncio.wait(
            {receiver_task, streamer_task, sequencer_task},
            return_when=asyncio.FIRST_COMPLETED,
        )

        for task in done:
            exc = task.exception()
            if exc is not None and not isinstance(exc, WebSocketDisconnect):
                logger.warning("WebSocket task exited with error for scene %s: %s", scene_id, exc)

        for task in pending:
            task.cancel()
        await sequencer.stop()
        self._connections.get(scene_id, set()).discard(websocket)

    async def _receive_events(self, websocket: WebSocket, sequencer: StepSequencer) -> None:
        while True:
            raw = await websocket.receive_text()
            try:
                event = PLAYBACK_ADAPTER.validate_python(json.loads(raw))
            except Exception:  # noqa: BLE001
                logger.warning("Invalid playback event payload: %s", raw)
                continue
            await sequencer.handle_event(event)

    async def _stream_bundles(self, websocket: WebSocket, sequencer: StepSequencer) -> None:
        while True:
            out = await sequencer.output_queue.get()
            await websocket.send_bytes(out.bundle_bytes)
