from uuid import uuid4

import pytest

from visualization_service.schema.scene_description import SceneDescription


def _valid_scene() -> dict:
    return {
        "scene_id": str(uuid4()),
        "session_id": str(uuid4()),
        "concept_title": "Test",
        "concept_summary": "Summary",
        "total_steps": 2,
        "steps": [
            {
                "step_index": 1,
                "step_label": "s1",
                "concept_type": "function_2d",
                "expression": "sin(x)",
                "narration": "n1",
                "domain": {"x": {"min": -1, "max": 1, "steps": 128}},
                "layer_mode": "replace",
                "transition": "fade_in",
                "hud_equation": "y=\\sin x",
            },
            {
                "step_index": 2,
                "step_label": "s2",
                "concept_type": "function_2d",
                "expression": "cos(x)",
                "narration": "n2",
                "domain": {"x": {"min": -1, "max": 1, "steps": 128}},
                "layer_mode": "add",
                "transition": "draw",
                "hud_equation": "y=\\cos x",
            },
        ],
    }


def test_scene_description_validates() -> None:
    scene = SceneDescription.model_validate(_valid_scene())
    assert scene.total_steps == 2


def test_scene_description_rejects_non_contiguous() -> None:
    raw = _valid_scene()
    raw["steps"][1]["step_index"] = 3
    with pytest.raises(Exception):  # noqa: PT011
        SceneDescription.model_validate(raw)
