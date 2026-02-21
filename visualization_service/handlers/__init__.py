from visualization_service.handlers.base import ConceptHandler
from visualization_service.handlers.complex_handler import ComplexHandler
from visualization_service.handlers.curve_handler import CurveHandler
from visualization_service.handlers.function_handler import FunctionHandler
from visualization_service.handlers.linalg_handler import LinalgHandler
from visualization_service.handlers.numerical_handler import NumericalHandler
from visualization_service.handlers.ode_handler import ODEHandler
from visualization_service.handlers.statistical_handler import StatisticalHandler
from visualization_service.handlers.surface_handler import SurfaceHandler
from visualization_service.handlers.topology_handler import TopologyHandler
from visualization_service.handlers.vector_field_handler import VectorFieldHandler
from visualization_service.schema.enums import ConceptType

HANDLER_REGISTRY: dict[ConceptType, ConceptHandler] = {
    ConceptType.FUNCTION_2D: FunctionHandler(),
    ConceptType.FUNCTION_3D: FunctionHandler(),
    ConceptType.EQUATION_BUILDUP: FunctionHandler(),
    ConceptType.SERIES_CONVERGENCE: FunctionHandler(),
    ConceptType.PARAMETRIC_CURVE_2D: CurveHandler(),
    ConceptType.PARAMETRIC_CURVE_3D: CurveHandler(),
    ConceptType.DERIVATIVE_TANGENT: CurveHandler(),
    ConceptType.PARAMETRIC_SURFACE: SurfaceHandler(),
    ConceptType.IMPLICIT_SURFACE: SurfaceHandler(),
    ConceptType.VECTOR_FIELD_2D: VectorFieldHandler(),
    ConceptType.VECTOR_FIELD_3D: VectorFieldHandler(),
    ConceptType.ODE_PHASE_PORTRAIT: ODEHandler(),
    ConceptType.GRADIENT_DESCENT: ODEHandler(),
    ConceptType.LINEAR_TRANSFORM: LinalgHandler(),
    ConceptType.EIGENSPACE_TRANSFORM: LinalgHandler(),
    ConceptType.COMPLEX_FUNCTION: ComplexHandler(),
    ConceptType.MANIFOLD: TopologyHandler(),
    ConceptType.DISTRIBUTION: StatisticalHandler(),
    ConceptType.RIEMANN_SUM: NumericalHandler(),
    ConceptType.LIMIT_APPROACH: NumericalHandler(),
}
