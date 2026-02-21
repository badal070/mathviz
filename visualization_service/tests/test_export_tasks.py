from __future__ import annotations

import msgpack

from visualization_service.tasks import export_tasks


def _bundle_bytes(step_index: int, layer_mode: str, layers: list[dict]) -> bytes:
    return msgpack.packb(
        {
            "v": 1,
            "step_index": step_index,
            "step_label": f"s{step_index}",
            "hud_equation": "x",
            "narration": "n",
            "layer_mode": layer_mode,
            "transition": "fade_in",
            "is_delta": False,
            "annotations": [],
            "layers": layers,
        },
        use_bin_type=True,
    )


class _Pipeline:
    def __init__(self, db: dict[str, bytes]) -> None:
        self.db = db
        self.keys: list[str] = []

    def get(self, key: str) -> None:
        self.keys.append(key)

    def execute(self) -> list[bytes | None]:
        return [self.db.get(k) for k in self.keys]


class _RedisFake:
    def __init__(self, db: dict[str, bytes]) -> None:
        self.db = db

    def scan_iter(self, match: str, count: int):
        _ = count
        prefix = match[:-1]
        for key in self.db:
            if key.startswith(prefix):
                yield key

    def pipeline(self, transaction: bool):
        _ = transaction
        return _Pipeline(self.db)

    def close(self) -> None:
        return None


def test_load_scene_bundles_sorted(monkeypatch) -> None:
    db = {
        "step_bundle:scene-1:2": _bundle_bytes(2, "add", [{"layer_id": "b"}]),
        "step_bundle:scene-1:1": _bundle_bytes(1, "replace", [{"layer_id": "a"}]),
    }

    monkeypatch.setattr(export_tasks, "Redis", type("_RedisFactory", (), {"from_url": staticmethod(lambda *_a, **_k: _RedisFake(db))}))
    bundles = export_tasks._load_scene_bundles("scene-1")
    assert [b["step_index"] for b in bundles] == [1, 2]


def test_reconstruct_frames_layer_modes() -> None:
    bundles = [
        {"step_index": 1, "step_label": "1", "hud_equation": "", "narration": "", "layer_mode": "replace", "layers": [{"layer_id": "a"}]},
        {"step_index": 2, "step_label": "2", "hud_equation": "", "narration": "", "layer_mode": "add", "layers": [{"layer_id": "b"}]},
        {"step_index": 3, "step_label": "3", "hud_equation": "", "narration": "", "layer_mode": "highlight", "layers": [{"layer_id": "h"}]},
    ]

    frames = export_tasks._reconstruct_frames(bundles)
    assert [len(f["layers"]) for f in frames] == [1, 2, 1]
    assert frames[1]["layers"][0]["layer_id"] == "a"
    assert frames[1]["layers"][1]["layer_id"] == "b"
    assert frames[2]["layers"][0]["layer_id"] == "h"


def test_export_common_uses_writer(monkeypatch, tmp_path) -> None:
    monkeypatch.setattr(
        export_tasks,
        "_load_scene_bundles",
        lambda _scene_id: [
            {
                "step_index": 1,
                "step_label": "1",
                "hud_equation": "",
                "narration": "",
                "layer_mode": "replace",
                "layers": [],
            }
        ],
    )
    monkeypatch.setattr(export_tasks, "_render_frames", lambda _frames, _w, _h: ["f1", "f2"])
    monkeypatch.setattr(export_tasks, "_artifact_path", lambda _scene_id, suffix: tmp_path / f"out.{suffix}")

    called: dict[str, int] = {}

    def _fake_write_mp4(frames, path, fps):
        called["frames"] = len(frames)
        called["fps"] = fps
        path.write_bytes(b"mp4")

    monkeypatch.setattr(export_tasks, "_write_mp4", _fake_write_mp4)

    result = export_tasks.export_video("scene-1", "user-1", resolution="801x601", fps=30)
    assert result["format"] == "mp4"
    assert result["status"] == "completed"
    assert result["resolution"] == "802x602"
    assert called == {"frames": 2, "fps": 30}
