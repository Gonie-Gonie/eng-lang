#[derive(Clone, Debug, PartialEq)]
pub struct SolverDiagnostics {
    pub status: String,
    pub convergence_status: String,
    pub failure: Option<SolverFailure>,
    pub iteration_count: usize,
    pub tolerance: f64,
    pub max_iterations: usize,
}

impl SolverDiagnostics {
    pub fn computed(tolerance: f64, max_iterations: usize, iteration_count: usize) -> Self {
        Self {
            status: "computed".to_owned(),
            convergence_status: "fixed_step_completed".to_owned(),
            failure: None,
            iteration_count,
            tolerance,
            max_iterations,
        }
    }

    pub fn failed(
        tolerance: f64,
        max_iterations: usize,
        iteration_count: usize,
        failure: SolverFailure,
    ) -> Self {
        Self {
            status: "failed".to_owned(),
            convergence_status: "failed".to_owned(),
            failure: Some(failure),
            iteration_count,
            tolerance,
            max_iterations,
        }
    }
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct SolverFailure {
    pub code: String,
    pub message: String,
}

impl SolverFailure {
    pub fn new(code: impl Into<String>, message: impl Into<String>) -> Self {
        Self {
            code: code.into(),
            message: message.into(),
        }
    }
}
