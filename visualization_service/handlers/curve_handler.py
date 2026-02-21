from __future__ import annotations

import sympy
from sympy.parsing.sympy_parser import (
    convert_xor,
    implicit_multiplication_application,
    parse_expr,
    standard_transformations,
)

from visualization_service.expression.parser import parse_expression
from visualization_service.expression.symbols import build_registry
from visualization_service.handlers._common import layer_meta_list, numeric_parameters, stable_hash
from visualization_service.handlers.base import ComputationSpec, ConceptHandler, LayerMetadata
from visualization_service.schema.enums import ConceptType
from visualization_service.schema.step_descriptor import StepDescriptor

TRANSFORMS = standard_transformations + (implicit_multiplication_application, convert_xor)


class CurveHandler(ConceptHandler):
    def build_computation_spec(self, step: StepDescriptor, domain_arrays: dict, parsed_asts: list[dict]) -> ComputationSpec:
        if step.domain is None:
            raise ValueError("Curve step requires domain")
        if not parsed_asts:
            raise ValueError("Curve step requires expression")

        params = numeric_parameters(step)
        domain = step.domain.clamped().model_dump(mode="json")

        if step.concept_type == ConceptType.DERIVATIVE_TANGENT:
            asts = self._build_derivative_tangent_asts(step, parsed_asts)
            entries = [
                {
                    "hash_key": stable_hash({"s": step.step_index, "i": i, "ast": ast, "d": domain}),
                    "ast": ast,
                    "domain": domain,
                    "concept_type": "curve_2d",
                    "layer_id": f"derivative_tangent_{step.step_index}_{i}",
                }
                for i, ast in enumerate(asts)
            ]
            return ComputationSpec(
                rust_function_name="batch_evaluate",
                request_payload={
                    "entries": entries,
                    "parameters": params,
                    "allow_non_finite": False,
                },
            )

        # Generic curve tracing (2D explicit). For parametric forms fallback to first component.
        payload = {
            "ast": parsed_asts[0],
            "domain": domain,
            "parameters": params,
            "discontinuity_threshold_factor": 10.0,
        }
        return ComputationSpec(rust_function_name="trace_curve", request_payload=payload)

    def build_layer_metadata(self, step: StepDescriptor) -> list[LayerMetadata]:
        if step.concept_type == ConceptType.DERIVATIVE_TANGENT:
            return layer_meta_list(step, [f"derivative_tangent_{step.step_index}_0", f"derivative_tangent_{step.step_index}_1"])
        return layer_meta_list(step, [f"curve_{step.step_index}"])

    def _build_derivative_tangent_asts(self, step: StepDescriptor, parsed_asts: list[dict]) -> list[dict]:
        expr_list = [step.expression] if isinstance(step.expression, str) else (step.expression or [])
        if not expr_list:
            return parsed_asts[:1]

        x0 = float(step.parameters.get("x0", 0.0)) if isinstance(step.parameters.get("x0", 0.0), (int, float)) else 0.0
        expr = parse_expr(expr_list[0], transformations=TRANSFORMS, evaluate=False)
        x = sympy.Symbol("x")
        derivative = sympy.diff(expr, x)
        y0 = float(expr.subs(x, x0))
        m = float(derivative.subs(x, x0))

        tangent = sympy.simplify(m * (x - x0) + y0)
        registry = build_registry({"x", "y", "z", "t"}, step.parameters)
        tangent_ast = parse_expression(str(tangent), registry)

        return [parsed_asts[0], tangent_ast]
