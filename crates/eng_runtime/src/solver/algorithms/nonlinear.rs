use crate::solver::{algorithms::linear::solve_dense_linear_system, euclidean_norm, SolverFailure};

#[derive(Clone, Debug, PartialEq)]
pub struct NewtonOptions {
    pub tolerance: f64,
    pub max_iterations: usize,
    pub finite_difference_step: f64,
    pub damping: f64,
    pub line_search_steps: usize,
}

impl Default for NewtonOptions {
    fn default() -> Self {
        Self {
            tolerance: 1e-9,
            max_iterations: 25,
            finite_difference_step: 1e-6,
            damping: 1.0,
            line_search_steps: 8,
        }
    }
}

#[derive(Clone, Debug, PartialEq)]
pub struct NewtonResult {
    pub values: Vec<f64>,
    pub residual_history: Vec<f64>,
    pub line_search_history: Vec<NewtonLineSearchStep>,
    pub linear_step_history: Vec<NewtonLinearStep>,
    pub jacobian_policy: String,
    pub largest_residual: Option<NewtonLargestResidual>,
    pub iteration_count: usize,
    pub convergence_status: String,
    pub failure: Option<SolverFailure>,
}

#[derive(Clone, Debug, PartialEq)]
pub struct NewtonLargestResidual {
    pub index: usize,
    pub value: f64,
    pub abs_value: f64,
}

#[derive(Clone, Debug, PartialEq)]
pub struct NewtonLineSearchStep {
    pub iteration: usize,
    pub scale: f64,
    pub trial_count: usize,
    pub residual_norm: f64,
}

#[derive(Clone, Debug, PartialEq)]
pub struct NewtonLinearStep {
    pub iteration: usize,
    pub residual_norm: f64,
    pub status: String,
    pub linear_condition_estimate: f64,
    pub linear_minimum_pivot_abs: f64,
    pub linear_maximum_pivot_abs: f64,
}

pub fn solve_newton<F>(
    initial: &[f64],
    options: &NewtonOptions,
    residual: F,
) -> Result<NewtonResult, SolverFailure>
where
    F: FnMut(&[f64]) -> Result<Vec<f64>, SolverFailure>,
{
    solve_newton_core(
        initial,
        options,
        residual,
        "finite_difference",
        |values, baseline_residuals, options, residual| {
            finite_difference_jacobian(values, baseline_residuals, options, residual)
        },
    )
}

pub fn solve_newton_with_jacobian<F, J>(
    initial: &[f64],
    options: &NewtonOptions,
    residual: F,
    mut jacobian: J,
) -> Result<NewtonResult, SolverFailure>
where
    F: FnMut(&[f64]) -> Result<Vec<f64>, SolverFailure>,
    J: FnMut(&[f64], &[f64]) -> Result<Vec<Vec<f64>>, SolverFailure>,
{
    solve_newton_core(
        initial,
        options,
        residual,
        "provided",
        |values, baseline_residuals, _options, _residual| jacobian(values, baseline_residuals),
    )
}

fn solve_newton_core<F, J>(
    initial: &[f64],
    options: &NewtonOptions,
    mut residual: F,
    jacobian_policy: &str,
    mut jacobian: J,
) -> Result<NewtonResult, SolverFailure>
where
    F: FnMut(&[f64]) -> Result<Vec<f64>, SolverFailure>,
    J: FnMut(&[f64], &[f64], &NewtonOptions, &mut F) -> Result<Vec<Vec<f64>>, SolverFailure>,
{
    validate_newton_options(initial, options)?;

    let mut values = initial.to_vec();
    let mut residual_values = residual(&values)?;
    validate_residual_layout(values.len(), &residual_values)?;
    let mut residual_norm = norm(&residual_values);
    let mut residual_history = vec![residual_norm];
    let mut line_search_history = Vec::new();
    let mut linear_step_history = Vec::new();
    if residual_norm <= options.tolerance {
        return Ok(build_newton_result(
            values,
            residual_history,
            line_search_history,
            linear_step_history,
            jacobian_policy,
            0,
            "newton_converged",
            None,
            &residual_values,
        ));
    }

    for iteration in 1..=options.max_iterations {
        let jacobian = jacobian(&values, &residual_values, options, &mut residual)?;
        validate_jacobian_layout(values.len(), &jacobian)?;
        let rhs = residual_values
            .iter()
            .map(|value| -value)
            .collect::<Vec<_>>();
        let linear = match solve_dense_linear_system(&jacobian, &rhs, options.tolerance) {
            Ok(linear) => linear,
            Err(failure) => {
                return Ok(build_newton_result(
                    values,
                    residual_history.clone(),
                    line_search_history.clone(),
                    linear_step_history.clone(),
                    jacobian_policy,
                    iteration,
                    "newton_linear_solve_failed",
                    Some(failure),
                    &residual_values,
                ));
            }
        };
        linear_step_history.push(NewtonLinearStep {
            iteration,
            residual_norm: linear.residual_norm,
            status: linear.status.clone(),
            linear_condition_estimate: linear.diagnostics.pivot_condition_estimate,
            linear_minimum_pivot_abs: linear.diagnostics.minimum_pivot_abs,
            linear_maximum_pivot_abs: linear.diagnostics.maximum_pivot_abs,
        });
        let step = linear.values;
        let accepted = match damped_step(&values, &step, residual_norm, options, &mut residual) {
            Ok(accepted) => accepted,
            Err(failure) => {
                return Ok(build_newton_result(
                    values,
                    residual_history.clone(),
                    line_search_history.clone(),
                    linear_step_history.clone(),
                    jacobian_policy,
                    iteration,
                    "newton_line_search_failed",
                    Some(failure),
                    &residual_values,
                ));
            }
        };
        line_search_history.push(NewtonLineSearchStep {
            iteration,
            scale: accepted.scale,
            trial_count: accepted.trial_count,
            residual_norm: accepted.residual_norm,
        });
        values = accepted.values;
        residual_values = accepted.residuals;
        residual_norm = accepted.residual_norm;
        residual_history.push(residual_norm);

        if residual_norm <= options.tolerance {
            return Ok(build_newton_result(
                values,
                residual_history,
                line_search_history.clone(),
                linear_step_history.clone(),
                jacobian_policy,
                iteration,
                "newton_converged",
                None,
                &residual_values,
            ));
        }
    }

    Ok(build_newton_result(
        values,
        residual_history,
        line_search_history,
        linear_step_history,
        jacobian_policy,
        options.max_iterations,
        "newton_not_converged",
        Some(SolverFailure::new(
            "E-NEWTON-NONCONVERGENCE",
            format!(
                "Newton solver did not converge after {} iteration(s); final residual norm was {}",
                options.max_iterations, residual_norm
            ),
        )),
        &residual_values,
    ))
}

fn build_newton_result(
    values: Vec<f64>,
    residual_history: Vec<f64>,
    line_search_history: Vec<NewtonLineSearchStep>,
    linear_step_history: Vec<NewtonLinearStep>,
    jacobian_policy: &str,
    iteration_count: usize,
    convergence_status: &str,
    failure: Option<SolverFailure>,
    residual_values: &[f64],
) -> NewtonResult {
    NewtonResult {
        values,
        residual_history,
        line_search_history,
        linear_step_history,
        jacobian_policy: jacobian_policy.to_owned(),
        largest_residual: largest_residual(residual_values),
        iteration_count,
        convergence_status: convergence_status.to_owned(),
        failure,
    }
}

fn validate_newton_options(initial: &[f64], options: &NewtonOptions) -> Result<(), SolverFailure> {
    if initial.is_empty() {
        return Err(SolverFailure::new(
            "E-NEWTON-SHAPE",
            "Newton solver requires at least one variable",
        ));
    }
    if initial.iter().any(|value| !value.is_finite()) {
        return Err(SolverFailure::new(
            "E-NEWTON-INITIAL-FINITE",
            "Newton solver initial values must be finite",
        ));
    }
    if options.max_iterations == 0 {
        return Err(SolverFailure::new(
            "E-NEWTON-ITERATIONS",
            "Newton solver requires max_iterations greater than zero",
        ));
    }
    if !options.tolerance.is_finite() || options.tolerance <= 0.0 {
        return Err(SolverFailure::new(
            "E-NEWTON-TOLERANCE",
            "Newton solver tolerance must be a positive finite number",
        ));
    }
    if !options.finite_difference_step.is_finite() || options.finite_difference_step <= 0.0 {
        return Err(SolverFailure::new(
            "E-NEWTON-FD-STEP",
            "Newton solver finite-difference step must be a positive finite number",
        ));
    }
    if !options.damping.is_finite() || options.damping <= 0.0 || options.damping > 1.0 {
        return Err(SolverFailure::new(
            "E-NEWTON-DAMPING",
            "Newton solver damping must be in the interval (0, 1]",
        ));
    }
    if options.line_search_steps == 0 {
        return Err(SolverFailure::new(
            "E-NEWTON-LINE-SEARCH-STEPS",
            "Newton solver line_search_steps must be greater than zero",
        ));
    }
    Ok(())
}

fn validate_residual_layout(expected: usize, residuals: &[f64]) -> Result<(), SolverFailure> {
    if residuals.len() != expected {
        return Err(SolverFailure::new(
            "E-NEWTON-RESIDUAL-LAYOUT",
            format!(
                "Newton residual vector length {} does not match variable count {}",
                residuals.len(),
                expected
            ),
        ));
    }
    if residuals.iter().any(|value| !value.is_finite()) {
        return Err(SolverFailure::new(
            "E-NEWTON-RESIDUAL-FINITE",
            "Newton residual evaluator returned a non-finite value",
        ));
    }
    Ok(())
}

fn validate_jacobian_layout(expected: usize, jacobian: &[Vec<f64>]) -> Result<(), SolverFailure> {
    if jacobian.len() != expected {
        return Err(SolverFailure::new(
            "E-NEWTON-JACOBIAN-LAYOUT",
            format!(
                "Newton Jacobian row count {} does not match variable count {}",
                jacobian.len(),
                expected
            ),
        ));
    }
    for (row_index, row) in jacobian.iter().enumerate() {
        if row.len() != expected {
            return Err(SolverFailure::new(
                "E-NEWTON-JACOBIAN-LAYOUT",
                format!(
                    "Newton Jacobian row {} length {} does not match variable count {}",
                    row_index,
                    row.len(),
                    expected
                ),
            ));
        }
        if row.iter().any(|value| !value.is_finite()) {
            return Err(SolverFailure::new(
                "E-NEWTON-JACOBIAN-FINITE",
                "Newton Jacobian evaluator returned a non-finite value",
            ));
        }
    }
    Ok(())
}

fn finite_difference_jacobian<F>(
    values: &[f64],
    baseline_residuals: &[f64],
    options: &NewtonOptions,
    residual: &mut F,
) -> Result<Vec<Vec<f64>>, SolverFailure>
where
    F: FnMut(&[f64]) -> Result<Vec<f64>, SolverFailure>,
{
    let n = values.len();
    let mut jacobian = vec![vec![0.0; n]; n];
    for column in 0..n {
        let mut perturbed = values.to_vec();
        let step = options.finite_difference_step * values[column].abs().max(1.0);
        if !step.is_finite() {
            return Err(SolverFailure::new(
                "E-NEWTON-FD-CANDIDATE-FINITE",
                "Newton finite-difference perturbation became non-finite",
            ));
        }
        perturbed[column] += step;
        ensure_finite_values(
            "E-NEWTON-FD-CANDIDATE-FINITE",
            "Newton finite-difference candidate",
            &perturbed,
        )?;
        let perturbed_residuals = residual(&perturbed)?;
        validate_residual_layout(n, &perturbed_residuals)?;
        for row in 0..n {
            jacobian[row][column] = (perturbed_residuals[row] - baseline_residuals[row]) / step;
        }
    }
    Ok(jacobian)
}

#[derive(Clone, Debug, PartialEq)]
struct AcceptedStep {
    values: Vec<f64>,
    residuals: Vec<f64>,
    residual_norm: f64,
    scale: f64,
    trial_count: usize,
}

fn damped_step<F>(
    values: &[f64],
    step: &[f64],
    current_residual_norm: f64,
    options: &NewtonOptions,
    residual: &mut F,
) -> Result<AcceptedStep, SolverFailure>
where
    F: FnMut(&[f64]) -> Result<Vec<f64>, SolverFailure>,
{
    let mut scale = options.damping;
    let attempts = options.line_search_steps.max(1);
    let mut best: Option<AcceptedStep> = None;
    let mut saw_nonfinite_candidate = false;
    for attempt_index in 0..attempts {
        let candidate = values
            .iter()
            .zip(step.iter())
            .map(|(value, delta)| value + scale * delta)
            .collect::<Vec<_>>();
        if candidate.iter().any(|value| !value.is_finite()) {
            saw_nonfinite_candidate = true;
            scale *= 0.5;
            continue;
        }
        let candidate_residuals = residual(&candidate)?;
        validate_residual_layout(values.len(), &candidate_residuals)?;
        let candidate_norm = norm(&candidate_residuals);
        let accepted = AcceptedStep {
            values: candidate,
            residuals: candidate_residuals,
            residual_norm: candidate_norm,
            scale,
            trial_count: attempt_index + 1,
        };
        if candidate_norm <= current_residual_norm {
            return Ok(accepted);
        }
        if best
            .as_ref()
            .is_none_or(|best| candidate_norm < best.residual_norm)
        {
            best = Some(accepted);
        }
        scale *= 0.5;
    }
    best.ok_or_else(|| {
        if saw_nonfinite_candidate {
            SolverFailure::new(
                "E-NEWTON-CANDIDATE-FINITE",
                "Newton line-search candidate became non-finite",
            )
        } else {
            SolverFailure::new(
                "E-NEWTON-LINE-SEARCH",
                "Newton solver could not evaluate any damped step candidate",
            )
        }
    })
}

fn norm(values: &[f64]) -> f64 {
    euclidean_norm(values)
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

fn largest_residual(values: &[f64]) -> Option<NewtonLargestResidual> {
    values
        .iter()
        .enumerate()
        .map(|(index, value)| NewtonLargestResidual {
            index,
            value: *value,
            abs_value: value.abs(),
        })
        .max_by(|left, right| {
            left.abs_value
                .partial_cmp(&right.abs_value)
                .unwrap_or(std::cmp::Ordering::Equal)
        })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn solves_scalar_nonlinear_system() {
        let result = solve_newton(&[1.0], &NewtonOptions::default(), |values| {
            Ok(vec![values[0] * values[0] - 2.0])
        })
        .unwrap();

        assert_eq!(result.convergence_status, "newton_converged");
        assert_eq!(result.jacobian_policy, "finite_difference");
        assert!(!result.linear_step_history.is_empty());
        assert!(result.linear_step_history[0].linear_condition_estimate >= 1.0);
        assert!(result.failure.is_none());
        assert!((result.values[0] - 2.0_f64.sqrt()).abs() < 1e-7);
        assert!(result.residual_history.last().copied().unwrap() <= 1e-9);
        assert_eq!(
            result
                .largest_residual
                .as_ref()
                .map(|residual| residual.index),
            Some(0)
        );
    }

    #[test]
    fn records_damped_line_search_steps() {
        let options = NewtonOptions {
            max_iterations: 1,
            ..Default::default()
        };
        let result = solve_newton(&[0.1], &options, |values| {
            Ok(vec![values[0] * values[0] - 2.0])
        })
        .unwrap();

        assert_eq!(result.line_search_history.len(), 1);
        let step = &result.line_search_history[0];
        assert_eq!(step.iteration, 1);
        assert!(step.scale < 1.0);
        assert!(step.trial_count > 1);
        assert_eq!(result.residual_history[1], step.residual_norm);
    }
    #[test]
    fn solves_two_variable_nonlinear_system() {
        let result = solve_newton(&[0.8, 2.1], &NewtonOptions::default(), |values| {
            let x = values[0];
            let y = values[1];
            Ok(vec![x + y - 3.0, x * x + y * y - 5.0])
        })
        .unwrap();

        assert_eq!(result.convergence_status, "newton_converged");
        assert!((result.values[0] - 1.0).abs() < 1e-7);
        assert!((result.values[1] - 2.0).abs() < 1e-7);
    }

    #[test]
    fn uses_supplied_jacobian_hook() {
        let mut jacobian_calls = 0;
        let result = solve_newton_with_jacobian(
            &[1.0],
            &NewtonOptions::default(),
            |values| Ok(vec![values[0] * values[0] - 2.0]),
            |values, _baseline_residuals| {
                jacobian_calls += 1;
                Ok(vec![vec![2.0 * values[0]]])
            },
        )
        .unwrap();

        assert_eq!(result.convergence_status, "newton_converged");
        assert_eq!(result.jacobian_policy, "provided");
        assert!(jacobian_calls > 0);
        assert!((result.values[0] - 2.0_f64.sqrt()).abs() < 1e-7);
    }

    #[test]
    fn rejects_invalid_supplied_jacobian_layout() {
        let failure = solve_newton_with_jacobian(
            &[0.0, 0.0],
            &NewtonOptions::default(),
            |values| Ok(vec![values[0] - 1.0, values[1] - 1.0]),
            |_values, _baseline_residuals| Ok(vec![vec![1.0]]),
        )
        .unwrap_err();

        assert_eq!(failure.code, "E-NEWTON-JACOBIAN-LAYOUT");
    }

    #[test]
    fn reports_singular_jacobian_failure() {
        let result = solve_newton(&[1.0], &NewtonOptions::default(), |_| Ok(vec![1.0])).unwrap();

        assert_eq!(result.convergence_status, "newton_linear_solve_failed");
        assert_eq!(result.iteration_count, 1);
        assert_eq!(
            result.failure.as_ref().map(|failure| failure.code.as_str()),
            Some("E-LINEAR-SINGULAR")
        );
        assert_eq!(
            result
                .largest_residual
                .as_ref()
                .map(|residual| residual.index),
            Some(0)
        );
    }

    #[test]
    fn reports_nonconvergence_with_failure_artifact() {
        let options = NewtonOptions {
            tolerance: 1e-15,
            max_iterations: 1,
            finite_difference_step: 1e-6,
            damping: 1.0,
            line_search_steps: 1,
        };
        let result = solve_newton(&[10.0], &options, |values| {
            Ok(vec![values[0] * values[0] - 2.0])
        })
        .unwrap();

        assert_eq!(result.convergence_status, "newton_not_converged");
        assert_eq!(
            result.failure.as_ref().map(|failure| failure.code.as_str()),
            Some("E-NEWTON-NONCONVERGENCE")
        );
        assert_eq!(
            result
                .largest_residual
                .as_ref()
                .map(|residual| residual.index),
            Some(0)
        );
    }

    #[test]
    fn rejects_nonfinite_initial_guess() {
        let failure =
            solve_newton(&[f64::NAN], &NewtonOptions::default(), |_| Ok(vec![0.0])).unwrap_err();

        assert_eq!(failure.code, "E-NEWTON-INITIAL-FINITE");
    }

    #[test]
    fn rejects_nonfinite_finite_difference_candidate() {
        let options = NewtonOptions {
            finite_difference_step: f64::MAX,
            ..Default::default()
        };
        let failure = solve_newton(&[2.0], &options, |values| Ok(vec![values[0]])).unwrap_err();

        assert_eq!(failure.code, "E-NEWTON-FD-CANDIDATE-FINITE");
    }

    #[test]
    fn reports_nonfinite_line_search_candidate() {
        let options = NewtonOptions {
            max_iterations: 1,
            line_search_steps: 1,
            ..Default::default()
        };
        let result = solve_newton_with_jacobian(
            &[f64::MAX],
            &options,
            |_| Ok(vec![-f64::MAX]),
            |_values, _baseline_residuals| Ok(vec![vec![1.0]]),
        )
        .unwrap();

        assert_eq!(result.convergence_status, "newton_line_search_failed");
        assert_eq!(result.iteration_count, 1);
        assert_eq!(
            result.failure.as_ref().map(|failure| failure.code.as_str()),
            Some("E-NEWTON-CANDIDATE-FINITE")
        );
    }

    #[test]
    fn rejects_invalid_newton_options() {
        let options = NewtonOptions {
            max_iterations: 0,
            ..Default::default()
        };
        let failure = solve_newton(&[1.0], &options, |values| Ok(values.to_vec())).unwrap_err();

        assert_eq!(failure.code, "E-NEWTON-ITERATIONS");

        let options = NewtonOptions {
            damping: 0.0,
            ..Default::default()
        };
        let failure = solve_newton(&[1.0], &options, |values| Ok(values.to_vec())).unwrap_err();

        assert_eq!(failure.code, "E-NEWTON-DAMPING");

        let options = NewtonOptions {
            line_search_steps: 0,
            ..Default::default()
        };
        let failure = solve_newton(&[1.0], &options, |values| Ok(values.to_vec())).unwrap_err();

        assert_eq!(failure.code, "E-NEWTON-LINE-SEARCH-STEPS");
    }
}
