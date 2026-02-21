from __future__ import annotations

from dataclasses import dataclass

CONSTANTS: dict[str, float] = {
    "pi": 3.141592653589793,
    "e": 2.718281828459045,
    "tau": 6.283185307179586,
    "golden_ratio": 1.618033988749895,
    "euler_mascheroni": 0.5772156649015329,
    "ln2": 0.6931471805599453,
    "ln10": 2.302585092994046,
    "sqrt2": 1.4142135623730951,
}


@dataclass(frozen=True)
class SymbolRegistry:
    allowed_symbols: set[str]
    constants: dict[str, float]


def build_registry(step_variables: set[str], parameters: dict[str, object]) -> SymbolRegistry:
    allowed = set(step_variables)
    allowed.update(parameters.keys())
    allowed.update(CONSTANTS.keys())
    return SymbolRegistry(allowed_symbols=allowed, constants=CONSTANTS.copy())
