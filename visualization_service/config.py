from __future__ import annotations

from pydantic import Field
from pydantic_settings import BaseSettings, SettingsConfigDict


class Settings(BaseSettings):
    model_config = SettingsConfigDict(env_file=".env", env_prefix="MATHVIZ_VIS_", extra="ignore")

    host: str = "0.0.0.0"
    port: int = 8082
    log_level: str = "info"
    uvicorn_workers: int = 1
    cors_origins: list[str] = Field(default_factory=lambda: ["*"])

    redis_url: str = "redis://localhost:6379/0"
    celery_broker_url: str = "redis://localhost:6379/1"
    celery_result_backend: str = "redis://localhost:6379/1"
    celery_task_soft_time_limit_seconds: int = 300
    celery_task_time_limit_seconds: int = 600
    geometry_cache_ttl_seconds: int = 7200
    step_bundle_ttl_seconds: int = 1800
    session_ttl_seconds: int = 86400

    ws_token_secret: str = "change-me"
    ws_token_ttl_seconds: int = 60

    max_batch_size: int = 4096
    default_step_dwell_ms: int = 4000
    max_surface_steps: int = 1024
    export_output_dir: str = "/tmp/mathviz_exports"
    export_default_width: int = 1280
    export_default_height: int = 720
    export_default_fps: int = 24
    export_max_frames: int = 2000

    rust_module_name: str = "mathviz_core"


settings = Settings()
