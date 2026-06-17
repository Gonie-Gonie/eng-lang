pub mod algorithms;
pub mod assembly;
pub mod behavior;
pub mod diagnostics;
pub mod evaluator;
pub mod plan;
pub mod residual;
pub mod result;
pub mod state_space;
pub mod thermal;

pub(crate) fn euclidean_norm(values: &[f64]) -> f64 {
    let mut scale = 0.0;
    let mut sum_squares = 1.0;
    let mut saw_nonzero = false;

    for abs_value in values.iter().map(|value| value.abs()) {
        if abs_value == 0.0 {
            continue;
        }
        if !abs_value.is_finite() {
            return abs_value;
        }
        saw_nonzero = true;
        if scale < abs_value {
            let ratio = scale / abs_value;
            sum_squares = 1.0 + sum_squares * ratio * ratio;
            scale = abs_value;
        } else {
            let ratio = abs_value / scale;
            sum_squares += ratio * ratio;
        }
    }

    if saw_nonzero {
        scale * sum_squares.sqrt()
    } else {
        0.0
    }
}

pub use algorithms::algebraic::{
    solve_linear_residual_graph, LinearResidualGraphSolution, LinearResidualVariableSolution,
};
pub use algorithms::dae::{
    initialize_algebraic_variables, solve_implicit_euler_dae, AlgebraicInitializationInput,
    DaeInput, DaeMassMatrix, DaeOptions, DaeResult, DaeSample, DaeStepReport, DaeTrajectory,
    DaeVariable,
};
pub use algorithms::dynamic_component::{
    solve_explicit_euler_with_algebraic, AlgebraicStepInput, DynamicComponentOptions,
    DynamicComponentResult, DynamicComponentStepDiagnostic, DynamicStepInput,
    ResidualGraphRhsEvaluator,
};
pub use algorithms::fixed_point::{solve_fixed_point, FixedPointOptions, FixedPointResult};
pub use algorithms::fixed_step::{solve_fixed_step_ode, FixedStepMethod, RhsSample};
pub use algorithms::nonlinear::{
    solve_newton, solve_newton_with_jacobian, NewtonLargestResidual, NewtonOptions, NewtonResult,
};
pub use behavior::{
    BehaviorExecutionProfile, BehaviorSignalContract, BehaviorWarning, DelayBehaviorNode,
    DelayBuffer, DelayEvaluation, DelayInitialHistoryPolicy, DelayInterpolationPolicy,
    DelayRelationshipArtifact, DelayRhsEvaluation, ExternalBehaviorArtifact,
    ExternalBehaviorContract, ExternalBehaviorDeterminism, ExternalBehaviorEvaluation,
    ExternalBehaviorKind, ExternalBehaviorNode, ExternalBehaviorProfilePolicy,
    ExternalBehaviorRhsEvaluation, PredictorBehaviorNode, PredictorContract,
    PredictorContractArtifact, PredictorDifferentiability, PredictorEvaluation,
    PredictorJacobianPolicy, PredictorRhsEvaluation, PredictorSolverPolicy,
};
pub use diagnostics::{SolverDiagnostics, SolverFailure};
pub use evaluator::{
    NamedDerivative, RhsEvaluator, RhsInput, RhsOutput, RhsStateInfo, StateSpaceRhsEvaluator,
};
pub use plan::{
    InputLayout, LayoutEntry, OutputLayout, ParameterLayout, SimulationPlan, SolverInput,
    SolverOptions, SolverPlan, SolverScalar, StateLayout, TimeGrid,
};
pub use residual::{
    LinearResidualSystem, NamedResidualValue, ResidualEquation, ResidualEvaluator,
    ResidualExpression, ResidualGraph, ResidualInput, ResidualOutput, ResidualScale,
    ResidualScaleOverride, ResidualSource, ResidualTerm, ResidualUnit, ResidualVariableRef,
    DEFAULT_RESIDUAL_TOLERANCE,
};
pub use result::{SolverOutput, SolverResult, StateTrajectory};
pub use state_space::{solve_continuous_state_space, solve_discrete_state_space};
pub use thermal::{solve_first_order_thermal, FirstOrderThermalModel};

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn euclidean_norm_handles_large_finite_values() {
        assert_eq!(euclidean_norm(&[]), 0.0);
        assert_eq!(euclidean_norm(&[0.0, 0.0]), 0.0);
        assert_eq!(euclidean_norm(&[f64::MAX]), f64::MAX);
        assert!(euclidean_norm(&[f64::MAX / 4.0, f64::MAX / 4.0]).is_finite());
    }
}
