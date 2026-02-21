from enum import Enum


class CoordinateSystem(str, Enum):
    CARTESIAN = "cartesian"
    CYLINDRICAL = "cylindrical"
    SPHERICAL = "spherical"


class LayerMode(str, Enum):
    ADD = "add"
    REPLACE = "replace"
    HIGHLIGHT = "highlight"


class TransitionType(str, Enum):
    FADE_IN = "fade_in"
    GROW = "grow"
    DRAW = "draw"
    TRANSFORM = "transform"
    HIGHLIGHT = "highlight"


class ConceptType(str, Enum):
    FUNCTION_2D = "function_2d"
    FUNCTION_3D = "function_3d"
    EQUATION_BUILDUP = "equation_buildup"
    SERIES_CONVERGENCE = "series_convergence"
    PARAMETRIC_CURVE_2D = "parametric_curve_2d"
    PARAMETRIC_CURVE_3D = "parametric_curve_3d"
    DERIVATIVE_TANGENT = "derivative_tangent"
    PARAMETRIC_SURFACE = "parametric_surface"
    IMPLICIT_SURFACE = "implicit_surface"
    VECTOR_FIELD_2D = "vector_field_2d"
    VECTOR_FIELD_3D = "vector_field_3d"
    ODE_PHASE_PORTRAIT = "ode_phase_portrait"
    GRADIENT_DESCENT = "gradient_descent"
    LINEAR_TRANSFORM = "linear_transform"
    EIGENSPACE_TRANSFORM = "eigenspace_transform"
    COMPLEX_FUNCTION = "complex_function"
    MANIFOLD = "manifold"
    DISTRIBUTION = "distribution"
    RIEMANN_SUM = "riemann_sum"
    LIMIT_APPROACH = "limit_approach"
