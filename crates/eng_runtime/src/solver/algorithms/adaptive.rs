use crate::solver::algorithms::fixed_step::RhsSample;
use crate::solver::{
    euclidean_norm, SolverDiagnostics, SolverFailure, SolverInput, SolverOutput, SolverResult,
    StateTrajectory,
};

#[derive(Clone, Debug, PartialEq)]
pub struct AdaptiveOdeOptions {
    pub tolerance: f64,
    pub initial_step_s: f64,
    pub min_step_s: f64,
    pub max_step_s: f64,
    pub safety_factor: f64,
    pub max_steps: usize,
}

impl Default for AdaptiveOdeOptions {
    fn default() -> Self {
        Self {
            tolerance: 1e-6,
            initial_step_s: 1.0,
            min_step_s: 1e-6,
            max_step_s: 60.0,
            safety_factor: 0.9,
            max_steps: 10_000,
        }
    }
}

#[derive(Clone, Debug, PartialEq)]
pub struct AdaptiveOdeStepReport {
    pub output_index: usize,
    pub start_time_s: f64,
    pub end_time_s: f64,
    pub dt_s: f64,
    pub error_norm: f64,
    pub status: String,
}

#[derive(Clone, Debug, PartialEq)]
pub struct AdaptiveOdeResult {
    pub solver_result: SolverResult,
    pub step_reports: Vec<AdaptiveOdeStepReport>,
}

struct HeunCandidate {
    state: Vec<f64>,
    error_norm: f64,
}

pub fn solve_adaptive_heun_ode<F>(
    input: &SolverInput,
    options: &AdaptiveOdeOptions,
    mut rhs: F,
) -> Result<AdaptiveOdeResult, SolverFailure>
where
    F: FnMut(RhsSample<'_>) -> Result<Vec<f64>, SolverFailure>,
{
    validate_adaptive_options(options)?;
    input.validate_layouts()?;
    if input.state_layout.is_empty() {
        return Err(SolverFailure::new(
            "E-SIM-SYSTEM-SHAPE-UNSUPPORTED",
            "adaptive solver input has no state variables",
        ));
    }

    let mut state = input.initial_state.clone();
    let mut current_time_s = input.time_grid.start_s;
    let mut next_dt_s = options.initial_step_s.min(options.max_step_s);
    let mut accepted_steps = 0usize;
    let mut attempted_steps = 0usize;
    let mut step_reports = Vec::new();
    let mut values_by_state = vec![Vec::with_capacity(input.time_grid.step_count + 1); state.len()];
    for (index, value) in state.iter().copied().enumerate() {
        values_by_state[index].push(value);
    }

    for output_index in 1..=input.time_grid.step_count {
        let target_time_s = input.time_grid.step_time_s(output_index);
        while current_time_s < target_time_s - f64::EPSILON {
            if attempted_steps >= options.max_steps {
                return Err(SolverFailure::new(
                    "E-ADAPTIVE-STEP-LIMIT",
                    "adaptive solver exceeded the configured maximum step attempts",
                ));
            }

            let remaining_s = target_time_s - current_time_s;
            let dt_s = next_dt_s.min(options.max_step_s).min(remaining_s);
            if dt_s <= 0.0 || !dt_s.is_finite() {
                return Err(SolverFailure::new(
                    "E-ADAPTIVE-STEP",
                    "adaptive solver produced an invalid timestep",
                ));
            }

            let candidate = heun_candidate(input, &mut rhs, current_time_s, &state, dt_s)?;
            attempted_steps += 1;
            let accept = candidate.error_norm <= options.tolerance || dt_s <= options.min_step_s;
            if accept {
                let start_time_s = current_time_s;
                current_time_s += dt_s;
                state = candidate.state;
                ensure_finite_values("E-ADAPTIVE-STATE-VALUE", "adaptive solver state", &state)?;
                accepted_steps += 1;
                step_reports.push(AdaptiveOdeStepReport {
                    output_index,
                    start_time_s,
                    end_time_s: current_time_s,
                    dt_s,
                    error_norm: candidate.error_norm,
                    status: "accepted".to_owned(),
                });
                next_dt_s = next_adaptive_dt(dt_s, candidate.error_norm, options);
            } else {
                step_reports.push(AdaptiveOdeStepReport {
                    output_index,
                    start_time_s: current_time_s,
                    end_time_s: current_time_s,
                    dt_s,
                    error_norm: candidate.error_norm,
                    status: "rejected_error_above_tolerance".to_owned(),
                });
                let reduced_dt_s =
                    next_adaptive_dt(dt_s, candidate.error_norm, options).min(dt_s * 0.5);
                if reduced_dt_s <= options.min_step_s && dt_s <= options.min_step_s {
                    return Err(SolverFailure::new(
                        "E-ADAPTIVE-TOLERANCE",
                        "adaptive solver could not satisfy tolerance at the minimum timestep",
                    ));
                }
                next_dt_s = reduced_dt_s.max(options.min_step_s);
            }
        }

        current_time_s = target_time_s;
        for (index, value) in state.iter().copied().enumerate() {
            values_by_state[index].push(value);
        }
    }

    let state_trajectories = input
        .state_layout
        .entries
        .iter()
        .zip(values_by_state)
        .map(|(entry, values)| StateTrajectory {
            name: entry.name.clone(),
            quantity_kind: entry.quantity_kind.clone(),
            canonical_unit: entry.canonical_unit.clone(),
            values,
        })
        .collect();
    let mut plan = input.plan.clone();
    plan.options.tolerance = options.tolerance;
    plan.options.max_iterations = options.max_steps;
    let solver_result = SolverResult {
        plan,
        time_grid: input.time_grid.clone(),
        state_layout: input.state_layout.clone(),
        output_layout: input.output_layout.clone(),
        output: SolverOutput {
            state_trajectories,
            algebraic_trajectories: Vec::new(),
        },
        diagnostics: SolverDiagnostics {
            status: "computed".to_owned(),
            convergence_status: "adaptive_heun_completed".to_owned(),
            failure: None,
            iteration_count: accepted_steps,
            tolerance: options.tolerance,
            max_iterations: options.max_steps,
        },
    };

    Ok(AdaptiveOdeResult {
        solver_result,
        step_reports,
    })
}

fn heun_candidate<F>(
    input: &SolverInput,
    rhs: &mut F,
    time_s: f64,
    state: &[f64],
    dt_s: f64,
) -> Result<HeunCandidate, SolverFailure>
where
    F: FnMut(RhsSample<'_>) -> Result<Vec<f64>, SolverFailure>,
{
    let k1 = rhs(RhsSample {
        time_s,
        state,
        inputs: &input.inputs,
        parameters: &input.parameters,
    })?;
    ensure_derivative_shape_and_values(&k1, state.len())?;

    let euler_state = offset_state(state, &k1, dt_s);
    ensure_finite_values(
        "E-ADAPTIVE-STATE-VALUE",
        "adaptive Euler candidate",
        &euler_state,
    )?;
    let k2 = rhs(RhsSample {
        time_s: time_s + dt_s,
        state: &euler_state,
        inputs: &input.inputs,
        parameters: &input.parameters,
    })?;
    ensure_derivative_shape_and_values(&k2, state.len())?;

    let heun_state = state
        .iter()
        .zip(k1.iter().zip(k2.iter()))
        .map(|(value, (k1, k2))| value + 0.5 * dt_s * (k1 + k2))
        .collect::<Vec<_>>();
    ensure_finite_values(
        "E-ADAPTIVE-STATE-VALUE",
        "adaptive Heun candidate",
        &heun_state,
    )?;
    let error = heun_state
        .iter()
        .zip(euler_state.iter())
        .map(|(heun, euler)| heun - euler)
        .collect::<Vec<_>>();
    let error_norm = euclidean_norm(&error);
    if !error_norm.is_finite() {
        return Err(SolverFailure::new(
            "E-ADAPTIVE-ERROR-VALUE",
            "adaptive solver error estimate is non-finite",
        ));
    }

    Ok(HeunCandidate {
        state: heun_state,
        error_norm,
    })
}

fn validate_adaptive_options(options: &AdaptiveOdeOptions) -> Result<(), SolverFailure> {
    if !options.tolerance.is_finite() || options.tolerance <= 0.0 {
        return Err(SolverFailure::new(
            "E-ADAPTIVE-OPTIONS",
            "adaptive solver tolerance must be a positive finite number",
        ));
    }
    if !options.initial_step_s.is_finite() || options.initial_step_s <= 0.0 {
        return Err(SolverFailure::new(
            "E-ADAPTIVE-OPTIONS",
            "adaptive solver initial step must be a positive finite number of seconds",
        ));
    }
    if !options.min_step_s.is_finite() || options.min_step_s <= 0.0 {
        return Err(SolverFailure::new(
            "E-ADAPTIVE-OPTIONS",
            "adaptive solver min step must be a positive finite number of seconds",
        ));
    }
    if !options.max_step_s.is_finite() || options.max_step_s < options.min_step_s {
        return Err(SolverFailure::new(
            "E-ADAPTIVE-OPTIONS",
            "adaptive solver max step must be finite and greater than or equal to min step",
        ));
    }
    if !options.safety_factor.is_finite() || options.safety_factor <= 0.0 {
        return Err(SolverFailure::new(
            "E-ADAPTIVE-OPTIONS",
            "adaptive solver safety factor must be a positive finite number",
        ));
    }
    if options.max_steps == 0 {
        return Err(SolverFailure::new(
            "E-ADAPTIVE-OPTIONS",
            "adaptive solver max_steps must be greater than zero",
        ));
    }
    Ok(())
}

fn next_adaptive_dt(dt_s: f64, error_norm: f64, options: &AdaptiveOdeOptions) -> f64 {
    if error_norm <= f64::EPSILON {
        return (dt_s * 2.0).min(options.max_step_s).max(options.min_step_s);
    }
    let factor = options.safety_factor * (options.tolerance / error_norm).sqrt();
    let clamped_factor = factor.clamp(0.2, 2.0);
    (dt_s * clamped_factor)
        .min(options.max_step_s)
        .max(options.min_step_s)
}

fn ensure_derivative_shape_and_values(
    derivative: &[f64],
    state_len: usize,
) -> Result<(), SolverFailure> {
    if derivative.len() != state_len {
        return Err(SolverFailure::new(
            "E-ADAPTIVE-RHS-LAYOUT",
            "adaptive solver RHS vector length does not match the state layout",
        ));
    }
    ensure_finite_values(
        "E-ADAPTIVE-RHS-VALUE",
        "adaptive solver RHS derivative",
        derivative,
    )
}

fn ensure_finite_values(code: &str, label: &str, values: &[f64]) -> Result<(), SolverFailure> {
    if values.iter().all(|value| value.is_finite()) {
        Ok(())
    } else {
        Err(SolverFailure::new(
            code,
            format!("{label} vector contains a non-finite value"),
        ))
    }
}

fn offset_state(state: &[f64], derivative: &[f64], scale: f64) -> Vec<f64> {
    state
        .iter()
        .zip(derivative.iter())
        .map(|(value, derivative)| value + derivative * scale)
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::solver::{
        InputLayout, LayoutEntry, OutputLayout, ParameterLayout, SimulationPlan, SolverOptions,
        SolverPlan, StateLayout, TimeGrid,
    };

    #[test]
    fn adaptive_heun_solves_fixed_output_grid_with_internal_substeps() {
        let input = adaptive_test_input();
        let result = solve_adaptive_heun_ode(
            &input,
            &AdaptiveOdeOptions {
                tolerance: 1e-4,
                initial_step_s: 0.5,
                min_step_s: 1e-4,
                max_step_s: 0.5,
                safety_factor: 0.9,
                max_steps: 100,
            },
            |sample| Ok(vec![-sample.state[0]]),
        )
        .unwrap();

        let trajectory = &result.solver_result.output.state_trajectories[0];
        assert_eq!(trajectory.values.len(), 3);
        assert!((trajectory.final_value().unwrap() - (-1.0_f64).exp()).abs() < 0.01);
        assert_eq!(
            result.solver_result.diagnostics.convergence_status,
            "adaptive_heun_completed"
        );
        assert!(result.solver_result.diagnostics.iteration_count > 2);
        assert!(result
            .step_reports
            .iter()
            .any(|report| report.status == "rejected_error_above_tolerance"));
    }

    #[test]
    fn adaptive_heun_rejects_nonfinite_rhs() {
        let input = adaptive_test_input();

        let failure = solve_adaptive_heun_ode(&input, &AdaptiveOdeOptions::default(), |_sample| {
            Ok(vec![f64::NAN])
        })
        .unwrap_err();

        assert_eq!(failure.code, "E-ADAPTIVE-RHS-VALUE");
    }

    #[test]
    fn adaptive_heun_reports_step_limit() {
        let input = adaptive_test_input();

        let failure = solve_adaptive_heun_ode(
            &input,
            &AdaptiveOdeOptions {
                tolerance: 1e-12,
                initial_step_s: 0.5,
                min_step_s: 1e-9,
                max_step_s: 0.5,
                safety_factor: 0.9,
                max_steps: 1,
            },
            |sample| Ok(vec![-sample.state[0]]),
        )
        .unwrap_err();

        assert_eq!(failure.code, "E-ADAPTIVE-STEP-LIMIT");
    }

    fn adaptive_test_input() -> SolverInput {
        SolverInput {
            plan: SolverPlan::new(
                "AdaptiveDecay",
                SimulationPlan {
                    states: vec!["x".to_owned()],
                    outputs: vec!["x".to_owned()],
                    inputs: Vec::new(),
                    parameters: Vec::new(),
                },
                SolverOptions {
                    method: "adaptive_heun".to_owned(),
                    timestep_s: 0.5,
                    tolerance: 1e-4,
                    max_iterations: 100,
                },
            ),
            time_grid: TimeGrid::fixed_step(1.0, 0.5).unwrap(),
            state_layout: StateLayout::new(vec![LayoutEntry::new(
                0,
                "x",
                "Dimensionless",
                "1",
                "1",
            )]),
            input_layout: InputLayout::default(),
            parameter_layout: ParameterLayout::default(),
            output_layout: OutputLayout {
                entries: vec![LayoutEntry::new(0, "x", "Dimensionless", "1", "1")],
            },
            initial_state: vec![1.0],
            inputs: Vec::new(),
            parameters: Vec::new(),
        }
    }
}
