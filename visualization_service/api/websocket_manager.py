from __future__ import annotations

import asyncio
import json

from fastapi import WebSocket
from pydantic import TypeAdapter

from visualization_service.api.auth import AuthError, verify_ws_token
from visualization_service.sequencer.playback_events import PlaybackEvent
from visualization_service.sequencer.step_sequencer import StepSequencer


class WebSocketManager:
    def __init__(self) -> None:
        self._connections: dict[str, WebSocket] = {}

    async def connect(self, scene_id: str, websocket: WebSocket, sequencer: StepSequencer) -> None:
        try:
            verify_ws_token(websocket, scene_id)
        except AuthError:
            await websocket.close(code=4008)
            return

        await websocket.accept()
        self._connections[scene_id] = websocket

        receiver_task = asyncio.create_task(self._receive_events(websocket, sequencer))
        streamer_task = asyncio.create_task(self._stream_bundles(websocket, sequencer))
        sequencer_task = asyncio.create_task(sequencer.start_streaming())

        done, pending = await asyncio.wait(
            {receiver_task, streamer_task, sequencer_task}, return_when=asyncio.FIRST_COMPLETED
        )
        for task in pending:
            task.cancel()
        await sequencer.stop()
        self._connections.pop(scene_id, None)

    async def _receive_events(self, websocket: WebSocket, sequencer: StepSequencer) -> None:
        adapter = TypeAdapter(PlaybackEvent)
        while True:
            raw = await websocket.receive_text()
            try:
                event = adapter.validate_python(json.loads(raw))
            except Exception:  # noqa: BLE001
                continue
            await sequencer.handle_event(event)

    async def _stream_bundles(self, websocket: WebSocket, sequencer: StepSequencer) -> None:
        while True:
            out = await sequencer.output_queue.get()
            await websocket.send_bytes(out.bundle_bytes)
