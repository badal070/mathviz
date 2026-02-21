from __future__ import annotations

try:
    from celery import Celery
except Exception:  # noqa: BLE001
    Celery = None


def create_celery() -> "Celery | None":
    if Celery is None:
        return None
    app = Celery("visualization_service", broker="redis://localhost:6379/1", backend="redis://localhost:6379/1")
    app.conf.task_serializer = "msgpack"
    app.conf.result_serializer = "msgpack"
    app.conf.accept_content = ["msgpack", "json"]
    app.conf.task_soft_time_limit = 60
    app.conf.task_time_limit = 120
    return app
