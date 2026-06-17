use super::diagnostics::SolverDiagnostics;
use super::plan::{SolverPlan, StateLayout, TimeGrid};

#[derive(Clone, Debug, PartialEq)]
pub struct SolverResult {
    pub plan: SolverPlan,
    pub time_grid: TimeGrid,
    pub state_layout: StateLayout,
    pub output: SolverOutput,
    pub diagnostics: SolverDiagnostics,
}

impl SolverResult {
    pub fn computed(
        plan: SolverPlan,
        time_grid: TimeGrid,
        state_layout: StateLayout,
        output: SolverOutput,
        iteration_count: usize,
    ) -> Self {
        let tolerance = plan.options.tolerance;
        let max_iterations = plan.options.max_iterations;
        Self {
            plan,
            time_grid,
            state_layout,
            output,
            diagnostics: SolverDiagnostics::computed(tolerance, max_iterations, iteration_count),
        }
    }

    pub fn single_state(&self) -> Option<&StateTrajectory> {
        (self.output.state_trajectories.len() == 1).then(|| &self.output.state_trajectories[0])
    }
}

#[derive(Clone, Debug, Default, PartialEq)]
pub struct SolverOutput {
    pub state_trajectories: Vec<StateTrajectory>,
    pub algebraic_trajectories: Vec<StateTrajectory>,
}

#[derive(Clone, Debug, PartialEq)]
pub struct StateTrajectory {
    pub name: String,
    pub quantity_kind: String,
    pub canonical_unit: String,
    pub values: Vec<f64>,
}

impl StateTrajectory {
    pub fn initial_value(&self) -> Option<f64> {
        self.values.first().copied()
    }

    pub fn final_value(&self) -> Option<f64> {
        self.values.last().copied()
    }
}
