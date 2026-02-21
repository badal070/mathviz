from __future__ import annotations

from redis.asyncio import Redis


class GeometryCache:
    def __init__(self, redis: Redis, geometry_ttl: int, bundle_ttl: int) -> None:
        self._redis = redis
        self._geometry_ttl = geometry_ttl
        self._bundle_ttl = bundle_ttl

    async def get_geometry(self, cache_key: str) -> bytes | None:
        data = await self._redis.get(f"geo:{cache_key}")
        return data if isinstance(data, bytes) else None

    async def set_geometry(self, cache_key: str, data: bytes, ttl: int | None = None) -> None:
        await self._redis.setex(f"geo:{cache_key}", ttl or self._geometry_ttl, data)

    async def get_step_bundle(self, scene_id: str, step_index: int) -> bytes | None:
        data = await self._redis.get(f"step_bundle:{scene_id}:{step_index}")
        return data if isinstance(data, bytes) else None

    async def set_step_bundle(self, scene_id: str, step_index: int, data: bytes, ttl: int | None = None) -> None:
        await self._redis.setex(f"step_bundle:{scene_id}:{step_index}", ttl or self._bundle_ttl, data)
