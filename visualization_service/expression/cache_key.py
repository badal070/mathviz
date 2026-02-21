from __future__ import annotations

import hashlib
import json
from typing import Any


def _normalize(obj: Any) -> Any:
    if isinstance(obj, float):
        return float(f"{obj:.10g}")
    if isinstance(obj, dict):
        return {k: _normalize(obj[k]) for k in sorted(obj.keys())}
    if isinstance(obj, list):
        return [_normalize(v) for v in obj]
    return obj


def compute_cache_key(ast_dict: dict[str, Any], domain_spec: dict[str, Any]) -> str:
    canonical = json.dumps({"ast": _normalize(ast_dict), "domain": _normalize(domain_spec)}, sort_keys=True)
    return hashlib.sha256(canonical.encode("utf-8")).hexdigest()[:32]
