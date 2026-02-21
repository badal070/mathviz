from __future__ import annotations

from contextlib import asynccontextmanager

from fastapi import FastAPI
from fastapi.middleware.cors import CORSMiddleware

from visualization_service.api.routes import router
from visualization_service.config import settings
from visualization_service.dependencies import redis_client


@asynccontextmanager
async def lifespan(app: FastAPI):
    _ = app
    client = redis_client()
    try:
        try:
            await client.ping()
        except Exception:
            pass
        yield
    finally:
        await client.close()


app = FastAPI(title="MathViz Visualization Service", lifespan=lifespan)
app.add_middleware(
    CORSMiddleware,
    allow_origins=settings.cors_origins,
    allow_credentials=True,
    allow_methods=["*"],
    allow_headers=["*"],
)
app.include_router(router)
