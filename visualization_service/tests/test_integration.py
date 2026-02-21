from visualization_service.main import app


def test_routes_registered() -> None:
    paths = {route.path for route in app.router.routes}
    assert "/health" in paths
    assert "/scenes" in paths
    assert "/scenes/{scene_id}/steps/{step_index}" in paths
    assert "/scenes/{scene_id}/state" in paths
    assert "/scenes/{scene_id}/reset" in paths
    assert "/scenes/{scene_id}/stream" in paths
