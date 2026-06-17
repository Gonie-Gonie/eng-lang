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
};
pub use algorithms::fixed_point::{solve_fixed_point, FixedPointOptions, FixedPointResult};
pub use algorithms::fixed_step::{solve_fixed_step_ode, FixedStepMethod, RhsSample};
pub use algorithms::nonlinear::{
    solve_newton, solve_newton_with_jacobian, NewtonLargestResidual, NewtonOptions, NewtonResult,
};
pub use behavior::{
    BehaviorExecutionProfile, BehaviorSignalContract, BehaviorWarning, DelayBehaviorNode,
    DelayBuffer, DelayEvaluation, DelayInitialHistoryPolicy, DelayInterpolationPolicy,
    DelayRelationshipArtifact, ExternalBehaviorArtifact, ExternalBehaviorContract,
    ExternalBehaviorDeterminism, ExternalBehaviorEvaluation, ExternalBehaviorKind,
    ExternalBehaviorNode, ExternalBehaviorProfilePolicy, PredictorBehaviorNode, PredictorContract,
    PredictorContractArtifact, PredictorDifferentiability, PredictorEvaluation,
    PredictorJacobianPolicy, PredictorSolverPolicy,
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
