from __future__ import annotations

import json
from dataclasses import asdict, dataclass
from datetime import UTC, datetime

from redis.asyncio import Redis


@dataclass
class SceneState:
    scene_id: str
    current_step: int
    total_steps: int
    is_playing: bool
    speed_multiplier: float
    accumulated_layer_ids: list[str]
    started_at: str

    @classmethod
    def new(cls, scene_id: str, total_steps: int) -> "SceneState":
        return cls(
            scene_id=scene_id,
            current_step=1,
            total_steps=total_steps,
            is_playing=True,
            speed_multiplier=1.0,
            accumulated_layer_ids=[],
            started_at=datetime.now(UTC).isoformat(),
        )


class SessionCache:
    def __init__(self, redis: Redis, ttl_seconds: int) -> None:
        self._redis = redis
        self._ttl = ttl_seconds

    async def get_scene_state(self, scene_id: str) -> SceneState | None:
        raw = await self._redis.get(f"scene_state:{scene_id}")
        if not raw:
            return None
        data = json.loads(raw.decode("utf-8") if isinstance(raw, bytes) else raw)
        return SceneState(**data)

    async def set_scene_state(self, scene_id: str, state: SceneState) -> None:
        await self._redis.setex(f"scene_state:{scene_id}", self._ttl, json.dumps(asdict(state)))

    async def get_conversation_context(self, session_id: str) -> list[dict]:
        raw = await self._redis.get(f"conversation:{session_id}")
        if not raw:
            return []
        data = json.loads(raw.decode("utf-8") if isinstance(raw, bytes) else raw)
        return data if isinstance(data, list) else []
