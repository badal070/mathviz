from __future__ import annotations

import base64
import hmac
import json
from hashlib import sha256
from time import time

from fastapi import WebSocket

from visualization_service.config import settings


class AuthError(ValueError):
    pass


def issue_ws_token(*, user_id: str, session_id: str, scene_id: str) -> str:
    payload = {
        "user_id": user_id,
        "session_id": session_id,
        "scene_id": scene_id,
        "expires_at": int(time()) + settings.ws_token_ttl_seconds,
    }
    raw = json.dumps(payload, separators=(",", ":")).encode("utf-8")
    payload_b64 = base64.urlsafe_b64encode(raw).decode("ascii").rstrip("=")
    signature = hmac.new(settings.ws_token_secret.encode("utf-8"), payload_b64.encode("utf-8"), sha256).hexdigest()
    return f"{payload_b64}.{signature}"


def verify_ws_token(websocket: WebSocket, scene_id: str) -> dict:
    token = websocket.query_params.get("token")
    if not token:
        raise AuthError("missing token")

    try:
        payload_b64, sig = token.rsplit(".", 1)
    except ValueError as exc:
        raise AuthError("malformed token") from exc

    expected = hmac.new(settings.ws_token_secret.encode("utf-8"), payload_b64.encode("utf-8"), sha256).hexdigest()
    if not hmac.compare_digest(expected, sig):
        raise AuthError("invalid signature")

    padded = payload_b64 + "=" * ((4 - len(payload_b64) % 4) % 4)
    try:
        payload = json.loads(base64.urlsafe_b64decode(padded.encode("ascii")).decode("utf-8"))
    except Exception as exc:  # noqa: BLE001
        raise AuthError("invalid payload encoding") from exc

    if int(payload.get("expires_at", 0)) < int(time()):
        raise AuthError("token expired")
    if payload.get("scene_id") != scene_id:
        raise AuthError("scene mismatch")
    return payload
