from __future__ import annotations

from functools import lru_cache

from redis.asyncio import Redis

from visualization_service.cache.geometry_cache import GeometryCache
from visualization_service.cache.session_cache import SessionCache
from visualization_service.config import settings
from visualization_service.rust_client import RustClient


@lru_cache(maxsize=1)
def redis_client() -> Redis:
    return Redis.from_url(
        settings.redis_url,
        decode_responses=False,
        socket_connect_timeout=0.25,
        socket_timeout=0.25,
        retry_on_timeout=False,
        health_check_interval=30,
    )


@lru_cache(maxsize=1)
def rust_client() -> RustClient:
    return RustClient.load()


def geometry_cache_dep() -> GeometryCache:
    return GeometryCache(redis_client(), settings.geometry_cache_ttl_seconds, settings.step_bundle_ttl_seconds)


def session_cache_dep() -> SessionCache:
    return SessionCache(redis_client(), settings.session_ttl_seconds)


def rust_client_dep() -> RustClient:
    return rust_client()
