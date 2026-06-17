use crate::solver::{
    algorithms::nonlinear::{solve_newton, NewtonOptions, NewtonResult},
    SolverFailure,
};

#[derive(Clone, Debug, PartialEq)]
pub struct DaeOptions {
    pub initial_time_s: f64,
    pub timestep_s: f64,
    pub step_count: usize,
    pub consistency_tolerance: f64,
    pub newton: NewtonOptions,
    pub mass_matrix: Option<DaeMassMatrix>,
}

impl Default for DaeOptions {
    fn default() -> Self {
        Self {
            initial_time_s: 0.0,
            timestep_s: 1.0,
            step_count: 1,
            consistency_tolerance: 1e-9,
            newton: NewtonOptions::default(),
            mass_matrix: None,
        }
    }
}

#[derive(Clone, Debug, PartialEq)]
pub struct DaeMassMatrix {
    pub rows: Vec<Vec<f64>>,
}

impl DaeMassMatrix {
    pub fn new(rows: Vec<Vec<f64>>) -> Self {
        Self { rows }
    }

    fn apply(&self, derivative: &[f64]) -> Result<Vec<f64>, SolverFailure> {
        if self.rows.len() != derivative.len()
            || self.rows.iter().any(|row| row.len() != derivative.len())
        {
            return Err(SolverFailure::new(
                "E-DAE-MASS-MATRIX-SHAPE",
                "DAE mass matrix must be square and match the state derivative length",
            ));
        }
        if self.rows.iter().flatten().any(|value| !value.is_finite()) {
            return Err(SolverFailure::new(
                "E-DAE-MASS-MATRIX-FINITE",
                "DAE mass matrix contains a non-finite value",
            ));
        }
        Ok(self
            .rows
            .iter()
            .map(|row| {
                row.iter()
                    .zip(derivative.iter())
                    .map(|(coefficient, value)| coefficient * value)
                    .sum()
            })
            .collect())
    }
}

#[derive(Clone, Debug, PartialEq)]
pub struct DaeVariable {
    pub name: String,
    pub initial_value: f64,
}

impl DaeVariable {
    pub fn new(name: impl Into<String>, initial_value: f64) -> Self {
        Self {
            name: name.into(),
            initial_value,
        }
    }
}

#[derive(Clone, Debug, PartialEq)]
pub struct DaeInput {
    pub states: Vec<DaeVariable>,
    pub initial_state_derivatives: Vec<f64>,
    pub algebraic: Vec<DaeVariable>,
    pub inputs: Vec<f64>,
    pub parameters: Vec<f64>,
}

#[derive(Clone, Debug, PartialEq)]
pub struct DaeSample<'a> {
    pub time_s: f64,
    pub state: &'a [f64],
    pub state_derivative: &'a [f64],
    pub mass_state_derivative: Option<&'a [f64]>,
    pub algebraic: &'a [f64],
    pub inputs: &'a [f64],
    pub parameters: &'a [f64],
}

#[derive(Clone, Debug, PartialEq)]
pub struct DaeTrajectory {
    pub name: String,
    pub values: Vec<f64>,
}

#[derive(Clone, Debug, PartialEq)]
pub struct DaeStepReport {
    pub step_index: usize,
    pub time_s: f64,
    pub newton: NewtonResult,
}

#[derive(Clone, Debug, PartialEq)]
pub struct DaeResult {
    pub state_trajectories: Vec<DaeTrajectory>,
    pub algebraic_trajectories: Vec<DaeTrajectory>,
    pub step_reports: Vec<DaeStepReport>,
    pub initial_residual_norm: f64,
    pub convergence_status: String,
    pub failure: Option<SolverFailure>,
}

pub fn solve_implicit_euler_dae<F>(
    input: &DaeInput,
    options: &DaeOptions,
    mut residual: F,
) -> Result<DaeResult, SolverFailure>
where
    F: FnMut(DaeSample<'_>) -> Result<Vec<f64>, SolverFailure>,
{
    validate_dae_input(input, options)?;

    let state_len = input.states.len();
    let algebraic_len = input.algebraic.len();
    let unknown_count = state_len + algebraic_len;
    let mut state = input
        .states
        .iter()
        .map(|variable| variable.initial_value)
        .collect::<Vec<_>>();
    let mut state_derivative = input.initial_state_derivatives.clone();
    let mut algebraic = input
        .algebraic
        .iter()
        .map(|variable| variable.initial_value)
        .collect::<Vec<_>>();

    let initial_mass_derivative = apply_mass_matrix(options, &state_derivative)?;
    let initial_residuals = residual(DaeSample {
        time_s: options.initial_time_s,
        state: &state,
        state_derivative: &state_derivative,
        mass_state_derivative: initial_mass_derivative.as_deref(),
        algebraic: &algebraic,
        inputs: &input.inputs,
        parameters: &input.parameters,
    })?;
    validate_dae_residual_layout(unknown_count, &initial_residuals)?;
    let initial_residual_norm = norm(&initial_residuals);
    if initial_residual_norm > options.consistency_tolerance {
        return Err(SolverFailure::new(
            "E-DAE-INCONSISTENT-INITIAL-CONDITIONS",
            format!(
                "DAE initial conditions are inconsistent; residual norm {} exceeds tolerance {}",
                initial_residual_norm, options.consistency_tolerance
            ),
        ));
    }

    let mut state_values = input
        .states
        .iter()
        .map(|variable| vec![variable.initial_value])
        .collect::<Vec<_>>();
    let mut algebraic_values = input
        .algebraic
        .iter()
        .map(|variable| vec![variable.initial_value])
        .collect::<Vec<_>>();
    let mut step_reports = Vec::new();

    for step_index in 1..=options.step_count {
        let previous_state = state.clone();
        let time_s = options.initial_time_s + options.timestep_s * step_index as f64;
        let mut guess = previous_state
            .iter()
            .zip(state_derivative.iter())
            .map(|(value, derivative)| value + options.timestep_s * derivative)
            .collect::<Vec<_>>();
        guess.extend(algebraic.iter().copied());

        let newton = solve_newton(&guess, &options.newton, |unknown| {
            let next_state = &unknown[..state_len];
            let next_algebraic = &unknown[state_len..];
            let next_derivative = next_state
                .iter()
                .zip(previous_state.iter())
                .map(|(next, previous)| (next - previous) / options.timestep_s)
                .collect::<Vec<_>>();
            let mass_derivative = apply_mass_matrix(options, &next_derivative)?;
            residual(DaeSample {
                time_s,
                state: next_state,
                state_derivative: &next_derivative,
                mass_state_derivative: mass_derivative.as_deref(),
                algebraic: next_algebraic,
                inputs: &input.inputs,
                parameters: &input.parameters,
            })
        })?;

        let failure = newton.failure.clone().map(|failure| {
            SolverFailure::new(
                "E-DAE-STEP-NONCONVERGENCE",
                format!(
                    "DAE implicit Euler step {} did not converge: {}",
                    step_index, failure.message
                ),
            )
        });
        let values = newton.values.clone();
        step_reports.push(DaeStepReport {
            step_index,
            time_s,
            newton,
        });
        if let Some(failure) = failure {
            return Ok(build_dae_result(
                input,
                state_values,
                algebraic_values,
                step_reports,
                initial_residual_norm,
                "dae_not_converged",
                Some(failure),
            ));
        }

        state = values[..state_len].to_vec();
        algebraic = values[state_len..].to_vec();
        state_derivative = state
            .iter()
            .zip(previous_state.iter())
            .map(|(next, previous)| (next - previous) / options.timestep_s)
            .collect();

        for (index, value) in state.iter().copied().enumerate() {
            state_values[index].push(value);
        }
        for (index, value) in algebraic.iter().copied().enumerate() {
            algebraic_values[index].push(value);
        }
    }

    Ok(build_dae_result(
        input,
        state_values,
        algebraic_values,
        step_reports,
        initial_residual_norm,
        "dae_converged",
        None,
    ))
}

pub fn initialize_algebraic_variables<F>(
    state: &[f64],
    state_derivative: &[f64],
    algebraic_guess: &[f64],
    inputs: &[f64],
    parameters: &[f64],
    time_s: f64,
    options: &NewtonOptions,
    mut residual: F,
) -> Result<NewtonResult, SolverFailure>
where
    F: FnMut(DaeSample<'_>) -> Result<Vec<f64>, SolverFailure>,
{
    if state.is_empty() || state.len() != state_derivative.len() {
        return Err(SolverFailure::new(
            "E-DAE-STATE-LAYOUT",
            "DAE algebraic initialization requires matching state and derivative vectors",
        ));
    }
    if algebraic_guess.is_empty() {
        return Err(SolverFailure::new(
            "E-DAE-ALGEBRAIC-SHAPE",
            "DAE algebraic initialization requires at least one algebraic variable",
        ));
    }

    solve_newton(algebraic_guess, options, |algebraic| {
        residual(DaeSample {
            time_s,
            state,
            state_derivative,
            mass_state_derivative: None,
            algebraic,
            inputs,
            parameters,
        })
    })
}

fn build_dae_result(
    input: &DaeInput,
    state_values: Vec<Vec<f64>>,
    algebraic_values: Vec<Vec<f64>>,
    step_reports: Vec<DaeStepReport>,
    initial_residual_norm: f64,
    convergence_status: &str,
    failure: Option<SolverFailure>,
) -> DaeResult {
    DaeResult {
        state_trajectories: input
            .states
            .iter()
            .zip(state_values)
            .map(|(variable, values)| DaeTrajectory {
                name: variable.name.clone(),
                values,
            })
            .collect(),
        algebraic_trajectories: input
            .algebraic
            .iter()
            .zip(algebraic_values)
            .map(|(variable, values)| DaeTrajectory {
                name: variable.name.clone(),
                values,
            })
            .collect(),
        step_reports,
        initial_residual_norm,
        convergence_status: convergence_status.to_owned(),
        failure,
    }
}

fn validate_dae_input(input: &DaeInput, options: &DaeOptions) -> Result<(), SolverFailure> {
    if input.states.is_empty() {
        return Err(SolverFailure::new(
            "E-DAE-STATE-SHAPE",
            "DAE implicit Euler requires at least one state variable",
        ));
    }
    if input.initial_state_derivatives.len() != input.states.len() {
        return Err(SolverFailure::new(
            "E-DAE-STATE-DERIVATIVE-LAYOUT",
            "DAE initial derivative vector length must match state count",
        ));
    }
    if options.step_count == 0 {
        return Err(SolverFailure::new(
            "E-DAE-STEP-COUNT",
            "DAE implicit Euler requires at least one step",
        ));
    }
    if !options.timestep_s.is_finite() || options.timestep_s <= 0.0 {
        return Err(SolverFailure::new(
            "E-DAE-TIMESTEP",
            "DAE implicit Euler timestep must be a positive finite number",
        ));
    }
    if !options.initial_time_s.is_finite() {
        return Err(SolverFailure::new(
            "E-DAE-INITIAL-TIME",
            "DAE initial time must be finite",
        ));
    }
    if !options.consistency_tolerance.is_finite() || options.consistency_tolerance <= 0.0 {
        return Err(SolverFailure::new(
            "E-DAE-CONSISTENCY-TOLERANCE",
            "DAE consistency tolerance must be a positive finite number",
        ));
    }
    if input
        .states
        .iter()
        .chain(input.algebraic.iter())
        .any(|variable| !variable.initial_value.is_finite())
        || input
            .initial_state_derivatives
            .iter()
            .any(|value| !value.is_finite())
    {
        return Err(SolverFailure::new(
            "E-DAE-INITIAL-FINITE",
            "DAE initial state, derivative, and algebraic values must be finite",
        ));
    }
    if let Some(mass_matrix) = &options.mass_matrix {
        mass_matrix.apply(&input.initial_state_derivatives)?;
    }
    Ok(())
}

fn validate_dae_residual_layout(expected: usize, residuals: &[f64]) -> Result<(), SolverFailure> {
    if residuals.len() != expected {
        return Err(SolverFailure::new(
            "E-DAE-RESIDUAL-LAYOUT",
            format!(
                "DAE residual vector length {} does not match unknown count {}",
                residuals.len(),
                expected
            ),
        ));
    }
    if residuals.iter().any(|value| !value.is_finite()) {
        return Err(SolverFailure::new(
            "E-DAE-RESIDUAL-FINITE",
            "DAE residual evaluator returned a non-finite value",
        ));
    }
    Ok(())
}

fn apply_mass_matrix(
    options: &DaeOptions,
    state_derivative: &[f64],
) -> Result<Option<Vec<f64>>, SolverFailure> {
    options
        .mass_matrix
        .as_ref()
        .map(|mass_matrix| mass_matrix.apply(state_derivative))
        .transpose()
}

fn norm(values: &[f64]) -> f64 {
    values.iter().map(|value| value * value).sum::<f64>().sqrt()
}

#[cfg(test)]
mod tests {
    use super::*;

    fn one_state_input(initial: f64, derivative: f64) -> DaeInput {
        DaeInput {
            states: vec![DaeVariable::new("x", initial)],
            initial_state_derivatives: vec![derivative],
            algebraic: Vec::new(),
            inputs: Vec::new(),
            parameters: Vec::new(),
        }
    }

    #[test]
    fn implicit_euler_solves_ode_residual_form() {
        let input = one_state_input(1.0, -1.0);
        let options = DaeOptions {
            step_count: 2,
            ..Default::default()
        };

        let result = solve_implicit_euler_dae(&input, &options, |sample| {
            Ok(vec![sample.state_derivative[0] + sample.state[0]])
        })
        .unwrap();

        assert_eq!(result.convergence_status, "dae_converged");
        assert!(result.failure.is_none());
        assert_eq!(result.state_trajectories[0].name, "x");
        assert!((result.state_trajectories[0].values[1] - 0.5).abs() < 1e-9);
        assert!((result.state_trajectories[0].values[2] - 0.25).abs() < 1e-9);
    }

    #[test]
    fn implicit_euler_solves_algebraic_variable() {
        let input = DaeInput {
            states: vec![DaeVariable::new("x", 1.0)],
            initial_state_derivatives: vec![-2.0],
            algebraic: vec![DaeVariable::new("z", 2.0)],
            inputs: Vec::new(),
            parameters: Vec::new(),
        };

        let result = solve_implicit_euler_dae(&input, &DaeOptions::default(), |sample| {
            Ok(vec![
                sample.state_derivative[0] + sample.algebraic[0],
                sample.algebraic[0] - 2.0 * sample.state[0],
            ])
        })
        .unwrap();

        assert_eq!(result.convergence_status, "dae_converged");
        assert!((result.state_trajectories[0].values[1] - (1.0 / 3.0)).abs() < 1e-9);
        assert!((result.algebraic_trajectories[0].values[1] - (2.0 / 3.0)).abs() < 1e-9);
        assert_eq!(result.step_reports.len(), 1);
    }

    #[test]
    fn mass_matrix_derivative_is_available_to_residual() {
        let input = one_state_input(1.0, -0.5);
        let options = DaeOptions {
            mass_matrix: Some(DaeMassMatrix::new(vec![vec![2.0]])),
            ..Default::default()
        };

        let result = solve_implicit_euler_dae(&input, &options, |sample| {
            Ok(vec![
                sample.mass_state_derivative.unwrap()[0] + sample.state[0],
            ])
        })
        .unwrap();

        assert_eq!(result.convergence_status, "dae_converged");
        assert!((result.state_trajectories[0].values[1] - (2.0 / 3.0)).abs() < 1e-9);
    }

    #[test]
    fn reports_step_nonconvergence_with_failure_artifact() {
        let input = one_state_input(1.0, -1.0);
        let options = DaeOptions {
            newton: NewtonOptions {
                tolerance: 1e-15,
                max_iterations: 1,
                finite_difference_step: 1e-6,
                damping: 1.0,
                line_search_steps: 1,
            },
            ..Default::default()
        };

        let result = solve_implicit_euler_dae(&input, &options, |sample| {
            Ok(vec![
                sample.state_derivative[0] + sample.state[0] * sample.state[0],
            ])
        })
        .unwrap();

        assert_eq!(result.convergence_status, "dae_not_converged");
        assert_eq!(
            result.failure.as_ref().map(|failure| failure.code.as_str()),
            Some("E-DAE-STEP-NONCONVERGENCE")
        );
        assert_eq!(result.step_reports.len(), 1);
        assert_eq!(
            result.step_reports[0].newton.convergence_status,
            "newton_not_converged"
        );
    }

    #[test]
    fn rejects_inconsistent_initial_conditions() {
        let input = one_state_input(1.0, 0.0);
        let failure = solve_implicit_euler_dae(&input, &DaeOptions::default(), |sample| {
            Ok(vec![sample.state_derivative[0] + sample.state[0]])
        })
        .unwrap_err();

        assert_eq!(failure.code, "E-DAE-INCONSISTENT-INITIAL-CONDITIONS");
    }

    #[test]
    fn initializes_algebraic_variables_with_newton() {
        let result = initialize_algebraic_variables(
            &[3.0],
            &[0.0],
            &[1.0],
            &[],
            &[],
            0.0,
            &NewtonOptions::default(),
            |sample| {
                Ok(vec![
                    sample.algebraic[0] * sample.algebraic[0] - sample.state[0],
                ])
            },
        )
        .unwrap();

        assert_eq!(result.convergence_status, "newton_converged");
        assert!((result.values[0] - 3.0_f64.sqrt()).abs() < 1e-7);
    }
}
