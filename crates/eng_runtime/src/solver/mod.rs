pub mod algorithms;
pub mod assembly;
pub mod diagnostics;
pub mod evaluator;
pub mod plan;
pub mod residual;
pub mod result;

pub use diagnostics::{SolverDiagnostics, SolverFailure};
pub use evaluator::{
    NamedDerivative, RhsEvaluator, RhsInput, RhsOutput, RhsStateInfo, StateSpaceRhsEvaluator,
};
pub use plan::{
    InputLayout, LayoutEntry, OutputLayout, ParameterLayout, SimulationPlan, SolverInput,
    SolverOptions, SolverPlan, SolverScalar, StateLayout, TimeGrid,
};
pub use residual::{
    NamedResidualValue, ResidualEquation, ResidualEvaluator, ResidualExpression, ResidualGraph,
    ResidualInput, ResidualOutput, ResidualScale, ResidualSource, ResidualTerm, ResidualUnit,
    ResidualVariableRef,
};
pub use result::{SolverOutput, SolverResult, StateTrajectory};
