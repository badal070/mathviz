from __future__ import annotations

from typing import Any

import sympy
from sympy.parsing.sympy_parser import (
    convert_xor,
    implicit_multiplication_application,
    parse_expr,
    standard_transformations,
)

from visualization_service.expression.symbols import SymbolRegistry


TRANSFORMS = standard_transformations + (implicit_multiplication_application, convert_xor)


class ExpressionParseError(ValueError):
    pass


def parse_expression(expr_str: str, registry: SymbolRegistry) -> dict[str, Any]:
    try:
        expr = parse_expr(expr_str, transformations=TRANSFORMS, evaluate=False)
    except Exception as exc:  # noqa: BLE001
        raise ExpressionParseError(f"Failed to parse expression '{expr_str}': {exc}") from exc

    free = {str(s) for s in expr.free_symbols}
    unknown = sorted(sym for sym in free if sym not in registry.allowed_symbols)
    if unknown:
        raise ExpressionParseError(f"Unknown symbols: {unknown}")

    for name, value in registry.constants.items():
        expr = expr.subs(sympy.Symbol(name), sympy.Float(value))

    # Also normalize common SymPy constants.
    expr = expr.subs(sympy.E, sympy.Float(registry.constants.get("e", float(sympy.E))))
    expr = expr.subs(sympy.pi, sympy.Float(registry.constants.get("pi", float(sympy.pi))))

    return _to_ast(expr)


def _to_ast(node: Any) -> dict[str, Any]:
    if isinstance(node, (sympy.Float, sympy.Integer, sympy.Rational)):
        return {"type": "literal", "value": float(node)}

    if isinstance(node, sympy.Symbol):
        return {"type": "variable", "name": str(node)}

    if isinstance(node, sympy.Add):
        children = [_to_ast(arg) for arg in node.args]
        if len(children) == 2:
            return {"type": "binary", "op": "add", "left": children[0], "right": children[1]}
        return {"type": "nary", "op": "sum", "children": children}

    if isinstance(node, sympy.Mul):
        children = [_to_ast(arg) for arg in node.args]
        if len(children) == 2:
            return {"type": "binary", "op": "mul", "left": children[0], "right": children[1]}
        return {"type": "nary", "op": "product", "children": children}

    if isinstance(node, sympy.Pow):
        return {
            "type": "binary",
            "op": "pow",
            "left": _to_ast(node.args[0]),
            "right": _to_ast(node.args[1]),
        }

    unary_map = {
        sympy.sin: "sin",
        sympy.cos: "cos",
        sympy.tan: "tan",
        sympy.asin: "asin",
        sympy.acos: "acos",
        sympy.atan: "atan",
        sympy.sinh: "sinh",
        sympy.cosh: "cosh",
        sympy.tanh: "tanh",
        sympy.sqrt: "sqrt",
        sympy.Abs: "abs",
        sympy.exp: "exp",
        sympy.log: "ln",
        sympy.floor: "floor",
        sympy.ceiling: "ceil",
        sympy.sign: "sign",
    }

    for fn, op in unary_map.items():
        if isinstance(node, fn):
            return {"type": "unary", "op": op, "child": _to_ast(node.args[0])}

    if isinstance(node, sympy.Mod):
        return {
            "type": "binary",
            "op": "mod",
            "left": _to_ast(node.args[0]),
            "right": _to_ast(node.args[1]),
        }

    if isinstance(node, sympy.atan2):
        return {
            "type": "binary",
            "op": "atan2",
            "left": _to_ast(node.args[0]),
            "right": _to_ast(node.args[1]),
        }

    raise ExpressionParseError(f"Unsupported node type: {type(node).__name__}")
