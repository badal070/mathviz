from __future__ import annotations

import logging
import os
from contextlib import asynccontextmanager

from fastapi import FastAPI
from fastapi.middleware.cors import CORSMiddleware

from visualization_service.api.routes import router
from visualization_service.config import settings
from visualization_service.dependencies import redis_client, rust_client

logger = logging.getLogger(__name__)


@asynccontextmanager
async def lifespan(app: FastAPI):
    _ = app
    logging.basicConfig(level=getattr(logging, settings.log_level.upper(), logging.INFO))
    redis = redis_client()

    try:
        try:
            await redis.ping()
        except Exception as exc:  # noqa: BLE001
            logger.warning("Redis ping failed at startup: %s", exc)

        try:
            rust = rust_client()
            rust.configure(max(1, os.cpu_count() or 1))
        except Exception as exc:  # noqa: BLE001
            logger.warning("Rust client startup check failed: %s", exc)

        yield
    finally:
        await redis.close()


def create_app() -> FastAPI:
    app = FastAPI(title="MathViz Visualization Service", lifespan=lifespan)
    app.add_middleware(
        CORSMiddleware,
        allow_origins=settings.cors_origins,
        allow_credentials=True,
        allow_methods=["*"],
        allow_headers=["*"],
    )
    app.include_router(router)
    return app


app = create_app()
