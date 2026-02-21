import pytest

from visualization_service.expression.cache_key import compute_cache_key
from visualization_service.expression.parser import ExpressionParseError, parse_expression
from visualization_service.expression.symbols import build_registry


def test_parse_expression_ok() -> None:
    reg = build_registry({"x"}, {})
    ast = parse_expression("sin(x)+2", reg)
    assert ast["type"] in {"binary", "nary"}


def test_parse_expression_unknown_symbol() -> None:
    reg = build_registry({"x"}, {})
    with pytest.raises(ExpressionParseError):
        parse_expression("sin(y)", reg)


def test_cache_key_stable() -> None:
    k1 = compute_cache_key({"a": 1.234567891234}, {"x": {"min": 0.0, "max": 1.0, "steps": 64}})
    k2 = compute_cache_key({"a": 1.234567891234}, {"x": {"min": 0.0, "max": 1.0, "steps": 64}})
    assert k1 == k2
