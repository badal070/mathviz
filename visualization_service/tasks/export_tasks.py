from __future__ import annotations

from visualization_service.tasks.celery_app import create_celery

celery_app = create_celery()


if celery_app is not None:

    @celery_app.task(name="export_pdf")
    def export_pdf(scene_id: str, user_id: str) -> dict:
        return {"scene_id": scene_id, "user_id": user_id, "format": "pdf", "status": "queued"}

    @celery_app.task(name="export_video")
    def export_video(scene_id: str, user_id: str, resolution: str) -> dict:
        return {
            "scene_id": scene_id,
            "user_id": user_id,
            "format": "mp4",
            "resolution": resolution,
            "status": "queued",
        }

    @celery_app.task(name="export_gif")
    def export_gif(scene_id: str, user_id: str) -> dict:
        return {"scene_id": scene_id, "user_id": user_id, "format": "gif", "status": "queued"}
