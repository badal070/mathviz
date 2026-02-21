from __future__ import annotations

import hmac
import json
from hashlib import sha256
from time import time

from fastapi import WebSocket

from visualization_service.config import settings


class AuthError(ValueError):
    pass


def verify_ws_token(websocket: WebSocket, scene_id: str) -> dict:
    token = websocket.query_params.get("token")
    if not token:
        raise AuthError("missing token")

    try:
        payload_b64, sig = token.rsplit(".", 1)
    except ValueError as exc:
        raise AuthError("malformed token") from exc

    expected = hmac.new(settings.ws_token_secret.encode(), payload_b64.encode(), sha256).hexdigest()
    if not hmac.compare_digest(expected, sig):
        raise AuthError("invalid signature")

    payload = json.loads(bytes.fromhex(payload_b64).decode("utf-8"))
    if int(payload.get("expires_at", 0)) < int(time()):
        raise AuthError("token expired")
    if payload.get("scene_id") != scene_id:
        raise AuthError("scene mismatch")
    return payload
