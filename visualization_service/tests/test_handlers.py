from visualization_service.handlers import HANDLER_REGISTRY
from visualization_service.schema.enums import ConceptType
from visualization_service.schema.step_descriptor import AxisSpec, DomainSpec, StepDescriptor


def _step(concept: ConceptType) -> StepDescriptor:
    return StepDescriptor(
        step_index=1,
        step_label="s",
        concept_type=concept,
        expression="sin(x)",
        narration="n",
        domain=DomainSpec(x=AxisSpec(min=-1, max=1, steps=128)),
        layer_mode="replace",
        transition="fade_in",
        hud_equation="y=\\sin x",
    )


def test_handlers_return_spec() -> None:
    for concept, handler in HANDLER_REGISTRY.items():
        step = _step(concept)
        if concept in {ConceptType.VECTOR_FIELD_2D, ConceptType.VECTOR_FIELD_3D}:
            parsed = [{"type": "variable", "name": "x"}, {"type": "variable", "name": "y"}, {"type": "literal", "value": 0.0}]
        elif concept in {ConceptType.LINEAR_TRANSFORM, ConceptType.EIGENSPACE_TRANSFORM}:
            step.expression = "[[1,0],[0,1]]"
            parsed = []
        else:
            parsed = [{"type": "variable", "name": "x"}]

        spec = handler.build_computation_spec(step, {}, parsed)
        assert isinstance(spec.rust_function_name, str)
