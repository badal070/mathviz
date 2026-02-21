from pydantic import Field
from pydantic_settings import BaseSettings, SettingsConfigDict


class Settings(BaseSettings):
    model_config = SettingsConfigDict(env_file=".env", env_prefix="MATHVIZ_VIS_")

    host: str = "0.0.0.0"
    port: int = 8082
    log_level: str = "info"
    cors_origins: list[str] = Field(default_factory=lambda: ["*"])

    redis_url: str = "redis://localhost:6379/0"
    geometry_cache_ttl_seconds: int = 7200
    step_bundle_ttl_seconds: int = 1800
    session_ttl_seconds: int = 86400

    ws_token_secret: str = "change-me"
    ws_token_ttl_seconds: int = 60

    max_batch_size: int = 4096
    default_step_dwell_ms: int = 4000
    max_surface_steps: int = 1024


settings = Settings()
