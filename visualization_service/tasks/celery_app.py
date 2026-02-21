from __future__ import annotations

from functools import lru_cache
from typing import Any

from visualization_service.config import settings

try:
    from celery import Celery
except Exception:  # noqa: BLE001
    Celery = None


@lru_cache(maxsize=1)
def create_celery() -> "Celery | None":
    if Celery is None:
        return None
    app = Celery("visualization_service", broker=settings.celery_broker_url, backend=settings.celery_result_backend)
    app.conf.update(
        task_serializer="json",
        result_serializer="json",
        accept_content=["json"],
        task_soft_time_limit=settings.celery_task_soft_time_limit_seconds,
        task_time_limit=settings.celery_task_time_limit_seconds,
        task_track_started=True,
        worker_prefetch_multiplier=1,
        task_acks_late=True,
        task_reject_on_worker_lost=True,
        imports=("visualization_service.tasks.export_tasks",),
        timezone="UTC",
        enable_utc=True,
    )
    return app


def enqueue_task(task_name: str, kwargs: dict[str, Any]) -> str:
    app = create_celery()
    if app is None:
        raise RuntimeError("Celery is not installed")
    result = app.send_task(task_name, kwargs=kwargs)
    return str(result.id)
